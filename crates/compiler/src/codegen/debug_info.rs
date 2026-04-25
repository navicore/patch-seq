//! DWARF debug info generation.
//!
//! Emits LLVM `!DICompileUnit`, `!DIFile`, `!DISubprogram`, and `!DILocation`
//! metadata records so a runtime panic backtrace resolves Seq frames to
//! `.seq:line:col`. The cost is metadata-only — no runtime overhead.
//!
//! Lifecycle within a program emission:
//!
//! 1. `dbg_init_program` — at the start of `codegen_program*`, allocate the
//!    compile unit, file, and shared subroutine type.
//! 2. `dbg_open_subprogram` — at the start of each emitted function, allocate
//!    a `DISubprogram` and stash its id in `current_dbg_subprogram_id`. The
//!    caller appends the returned `!dbg !N` suffix to the `define` line.
//! 3. `dbg_call_suffix` — for each emitted `call` with a span, allocates a
//!    `DILocation` and returns the `, !dbg !N` suffix to append.
//! 4. `dbg_close_subprogram` — clears `current_dbg_subprogram_id` after a
//!    function is fully emitted.
//! 5. `dbg_emit_module_metadata` — at the end of IR emission, dumps all
//!    accumulated metadata records and the module flags.
//!
//! When `dbg_source` is `None`, every method is a no-op and the IR is
//! identical to a build without debug info.

use super::CodeGen;
use crate::ast::Span;
use std::fmt::Write as _;

impl CodeGen {
    /// True when debug info should be emitted. Cheap, called per call-site.
    pub(super) fn dbg_enabled(&self) -> bool {
        self.dbg_source.is_some()
    }

    /// Allocate the next metadata id.
    fn dbg_alloc_id(&mut self) -> usize {
        self.dbg_md_counter += 1;
        self.dbg_md_counter
    }

    /// Initialise the compile unit, file, and shared subroutine type.
    /// Idempotent — safe to call once per program emission.
    pub(super) fn dbg_init_program(&mut self) {
        if !self.dbg_enabled() || self.dbg_cu_id.is_some() {
            return;
        }

        let source = self.dbg_source.as_ref().expect("dbg_enabled checked");
        let abs = std::fs::canonicalize(source).unwrap_or_else(|_| source.clone());
        let (filename, directory) = match (abs.file_name(), abs.parent()) {
            (Some(name), Some(dir)) => (
                name.to_string_lossy().into_owned(),
                dir.to_string_lossy().into_owned(),
            ),
            _ => (abs.to_string_lossy().into_owned(), String::new()),
        };

        let file_id = self.dbg_alloc_id();
        let cu_id = self.dbg_alloc_id();
        let sub_ty_id = self.dbg_alloc_id();
        // Reserve module-flag IDs through the same counter so they never
        // collide with later subprogram/location records on big programs.
        let dwarf_ver_id = self.dbg_alloc_id();
        let debug_info_ver_id = self.dbg_alloc_id();

        let _ = writeln!(
            &mut self.dbg_metadata,
            "!{} = !DIFile(filename: \"{}\", directory: \"{}\")",
            file_id,
            escape_md(&filename),
            escape_md(&directory),
        );
        let _ = writeln!(
            &mut self.dbg_metadata,
            "!{} = distinct !DICompileUnit(language: DW_LANG_C, file: !{}, \
             producer: \"seqc\", isOptimized: false, runtimeVersion: 0, \
             emissionKind: FullDebug)",
            cu_id, file_id,
        );
        let _ = writeln!(
            &mut self.dbg_metadata,
            "!{} = !DISubroutineType(types: !{{null}})",
            sub_ty_id,
        );

        self.dbg_file_id = Some(file_id);
        self.dbg_cu_id = Some(cu_id);
        self.dbg_subroutine_type_id = Some(sub_ty_id);
        self.dbg_module_flag_ids = Some((dwarf_ver_id, debug_info_ver_id));
    }

    /// Allocate a `DISubprogram` for the function currently being emitted.
    ///
    /// Returns the suffix to append to the `define` header line — either
    /// ` !dbg !N` or `""` when debug info is disabled. Stashes the id in
    /// `current_dbg_subprogram_id` so subsequent call-site emissions can
    /// reference it as their scope.
    pub(super) fn dbg_open_subprogram(&mut self, name: &str, line: usize) -> String {
        if !self.dbg_enabled() {
            return String::new();
        }
        // Bootstrap on first use — keeps callers from having to remember.
        self.dbg_init_program();

        let (Some(file_id), Some(cu_id), Some(sub_ty_id)) = (
            self.dbg_file_id,
            self.dbg_cu_id,
            self.dbg_subroutine_type_id,
        ) else {
            return String::new();
        };

        let sp_id = self.dbg_alloc_id();
        // DWARF lines are 1-indexed; spans are 0-indexed internally.
        let dwarf_line = line.saturating_add(1);
        let _ = writeln!(
            &mut self.dbg_metadata,
            "!{} = distinct !DISubprogram(name: \"{}\", scope: !{}, \
             file: !{}, line: {}, type: !{}, scopeLine: {}, \
             spFlags: DISPFlagDefinition, unit: !{})",
            sp_id,
            escape_md(name),
            file_id,
            file_id,
            dwarf_line,
            sub_ty_id,
            dwarf_line,
            cu_id,
        );

        self.current_dbg_subprogram_id = Some(sp_id);
        format!(" !dbg !{}", sp_id)
    }

    /// Clear the current subprogram. Called when a function definition is
    /// fully emitted.
    pub(super) fn dbg_close_subprogram(&mut self) {
        self.current_dbg_subprogram_id = None;
    }

    /// Suffix to append to a `call` instruction so its address resolves
    /// back to the originating .seq line. Returns `""` when debug info is
    /// disabled, no subprogram is open, or the statement has no span.
    pub(super) fn dbg_call_suffix(&mut self, span: Option<&Span>) -> String {
        if !self.dbg_enabled() {
            return String::new();
        }
        let (Some(scope), Some(sp)) = (self.current_dbg_subprogram_id, span) else {
            return String::new();
        };
        let loc_id = self.dbg_alloc_id();
        let _ = writeln!(
            &mut self.dbg_metadata,
            "!{} = !DILocation(line: {}, column: {}, scope: !{})",
            loc_id,
            sp.line.saturating_add(1),
            sp.column.saturating_add(1),
            scope,
        );
        format!(", !dbg !{}", loc_id)
    }

    /// Append accumulated debug metadata and module flags to the final IR.
    /// Called once at the end of `codegen_program*` after `self.output`
    /// has been concatenated.
    pub(super) fn dbg_emit_module_metadata(&self, ir: &mut String) {
        let (Some(cu_id), Some((dwarf_ver_id, debug_info_ver_id))) =
            (self.dbg_cu_id, self.dbg_module_flag_ids)
        else {
            return;
        };
        if !self.dbg_enabled() {
            return;
        }
        let _ = writeln!(ir);
        let _ = writeln!(ir, "!llvm.dbg.cu = !{{!{}}}", cu_id);
        // Dwarf Version + Debug Info Version are required by the verifier
        // when any !dbg metadata exists in the module.
        let _ = writeln!(
            ir,
            "!llvm.module.flags = !{{!{}, !{}}}",
            dwarf_ver_id, debug_info_ver_id,
        );
        let _ = writeln!(
            ir,
            "!{} = !{{i32 7, !\"Dwarf Version\", i32 4}}",
            dwarf_ver_id,
        );
        let _ = writeln!(
            ir,
            "!{} = !{{i32 2, !\"Debug Info Version\", i32 3}}",
            debug_info_ver_id,
        );
        ir.push_str(&self.dbg_metadata);
    }
}

/// Escape a string for embedding as a metadata string literal.
/// LLVM IR strings use C-style escapes; backslashes and quotes are the
/// two characters we have to be careful about for paths.
fn escape_md(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
