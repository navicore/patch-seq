//! `seqc venv` subcommand: create a virtualenv-like directory with activate
//! scripts for bash, fish, and csh.

use std::path::{Path, PathBuf};
use std::process;

pub(crate) fn run_venv(name: &Path) {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    // Helper to cleanup partially created venv on failure
    fn cleanup_and_exit(venv_path: &Path, msg: &str) -> ! {
        eprintln!("{}", msg);
        if let Err(e) = std::fs::remove_dir_all(venv_path) {
            eprintln!("Warning: failed to cleanup {}: {}", venv_path.display(), e);
        }
        process::exit(1);
    }

    // Get absolute path for the venv, normalizing to remove trailing slashes
    let venv_path: PathBuf = if name.is_absolute() {
        name.components().collect()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(name)
            .components()
            .collect()
    };

    // Check if directory already exists
    if venv_path.exists() {
        eprintln!("Error: {} already exists", venv_path.display());
        process::exit(1);
    }

    // Create directory structure
    let bin_dir = venv_path.join("bin");
    if let Err(e) = fs::create_dir_all(&bin_dir) {
        eprintln!("Error creating directory {}: {}", bin_dir.display(), e);
        process::exit(1);
    }

    // Find current executable's directory
    let current_exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(e) => {
            cleanup_and_exit(
                &venv_path,
                &format!("Error finding current executable: {}", e),
            );
        }
    };
    let exe_dir = match current_exe.parent() {
        Some(dir) => dir,
        None => {
            cleanup_and_exit(
                &venv_path,
                "Error: could not determine executable directory",
            );
        }
    };

    // Copy binaries
    let binaries = ["seqc", "seqr", "seq-lsp"];
    let mut copied_count = 0;
    for binary in binaries {
        let src = exe_dir.join(binary);
        let dst = bin_dir.join(binary);

        if !src.exists() {
            eprintln!("Warning: {} not found, skipping", src.display());
            continue;
        }

        if let Err(e) = fs::copy(&src, &dst) {
            cleanup_and_exit(&venv_path, &format!("Error copying {}: {}", binary, e));
        }

        // Set executable permissions on Unix
        #[cfg(unix)]
        if let Err(e) = fs::set_permissions(&dst, fs::Permissions::from_mode(0o755)) {
            eprintln!("Warning: could not set permissions on {}: {}", binary, e);
        }

        println!("  Copied {}", binary);
        copied_count += 1;
    }

    if copied_count == 0 {
        cleanup_and_exit(
            &venv_path,
            &format!("Error: no seq binaries found in {}", exe_dir.display()),
        );
    }

    // Generate activate scripts
    // Use components().last() instead of file_name() to handle trailing slashes
    let venv_name = venv_path
        .components()
        .next_back()
        .and_then(|c| c.as_os_str().to_str())
        .unwrap_or("seq-venv");

    if let Err(e) = generate_activate_bash(&venv_path, venv_name) {
        cleanup_and_exit(
            &venv_path,
            &format!("Error generating activate script: {}", e),
        );
    }

    if let Err(e) = generate_activate_fish(&venv_path, venv_name) {
        cleanup_and_exit(
            &venv_path,
            &format!("Error generating activate.fish script: {}", e),
        );
    }

    if let Err(e) = generate_activate_csh(&venv_path, venv_name) {
        cleanup_and_exit(
            &venv_path,
            &format!("Error generating activate.csh script: {}", e),
        );
    }

    println!("\nCreated virtual environment at {}", venv_path.display());
    println!("\nTo activate, run:");
    println!("  source {}/bin/activate", venv_path.display());
}

fn generate_activate_bash(venv_path: &Path, venv_name: &str) -> std::io::Result<()> {
    use std::fs;

    let script = format!(
        r#"# This file must be sourced with "source activate" from bash/zsh.
# It cannot be run directly.

deactivate () {{
    # Reset PATH
    if [ -n "${{_OLD_VIRTUAL_PATH:-}}" ]; then
        PATH="${{_OLD_VIRTUAL_PATH}}"
        export PATH
        unset _OLD_VIRTUAL_PATH
    fi

    # Reset prompt
    if [ -n "${{_OLD_VIRTUAL_PS1:-}}" ]; then
        PS1="${{_OLD_VIRTUAL_PS1}}"
        export PS1
        unset _OLD_VIRTUAL_PS1
    fi

    unset SEQ_VIRTUAL_ENV

    if [ ! "${{1:-}}" = "nondestructive" ]; then
        unset -f deactivate
    fi
}}

# Unset irrelevant variables
deactivate nondestructive

SEQ_VIRTUAL_ENV="{venv_path}"
export SEQ_VIRTUAL_ENV

_OLD_VIRTUAL_PATH="$PATH"
PATH="$SEQ_VIRTUAL_ENV/bin:$PATH"
export PATH

_OLD_VIRTUAL_PS1="${{PS1:-}}"
PS1="({venv_name}) ${{PS1:-}}"
export PS1
"#,
        venv_path = venv_path.display(),
        venv_name = venv_name
    );

    fs::write(venv_path.join("bin").join("activate"), script)?;
    println!("  Generated bin/activate");
    Ok(())
}

fn generate_activate_fish(venv_path: &Path, venv_name: &str) -> std::io::Result<()> {
    use std::fs;

    let script = format!(
        r#"# This file must be sourced with "source activate.fish" from fish.

function deactivate -d "Exit virtual environment"
    # Reset PATH
    if set -q _OLD_VIRTUAL_PATH
        set -gx PATH $_OLD_VIRTUAL_PATH
        set -e _OLD_VIRTUAL_PATH
    end

    # Reset prompt
    if functions -q _old_fish_prompt
        functions -e fish_prompt
        functions -c _old_fish_prompt fish_prompt
        functions -e _old_fish_prompt
    end

    set -e SEQ_VIRTUAL_ENV

    if test "$argv[1]" != "nondestructive"
        functions -e deactivate
    end
end

# Unset irrelevant variables
deactivate nondestructive

set -gx SEQ_VIRTUAL_ENV "{venv_path}"

set -gx _OLD_VIRTUAL_PATH $PATH
set -gx PATH "$SEQ_VIRTUAL_ENV/bin" $PATH

# Save current prompt
if functions -q fish_prompt
    functions -c fish_prompt _old_fish_prompt
end

function fish_prompt
    printf "({venv_name}) "
    _old_fish_prompt
end
"#,
        venv_path = venv_path.display(),
        venv_name = venv_name
    );

    fs::write(venv_path.join("bin").join("activate.fish"), script)?;
    println!("  Generated bin/activate.fish");
    Ok(())
}

fn generate_activate_csh(venv_path: &Path, venv_name: &str) -> std::io::Result<()> {
    use std::fs;

    let script = format!(
        r#"# This file must be sourced with "source activate.csh" from csh/tcsh.

alias deactivate 'if ($?_OLD_VIRTUAL_PATH) then; setenv PATH "$_OLD_VIRTUAL_PATH"; unsetenv _OLD_VIRTUAL_PATH; endif; unsetenv SEQ_VIRTUAL_ENV; unalias deactivate'

setenv SEQ_VIRTUAL_ENV "{venv_path}"

setenv _OLD_VIRTUAL_PATH "$PATH"
setenv PATH "$SEQ_VIRTUAL_ENV/bin:$PATH"

set prompt = "({venv_name}) $prompt"
"#,
        venv_path = venv_path.display(),
        venv_name = venv_name
    );

    fs::write(venv_path.join("bin").join("activate.csh"), script)?;
    println!("  Generated bin/activate.csh");
    Ok(())
}
