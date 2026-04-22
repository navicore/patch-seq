//! Documentation strings for built-in words, plus the public `builtin_doc`
//! and `builtin_docs` accessors.

use std::collections::HashMap;
use std::sync::LazyLock;

/// Get documentation for a built-in word
pub fn builtin_doc(name: &str) -> Option<&'static str> {
    BUILTIN_DOCS.get(name).copied()
}

/// Get all built-in word documentation (cached with LazyLock for performance)
pub fn builtin_docs() -> &'static HashMap<&'static str, &'static str> {
    &BUILTIN_DOCS
}

/// Lazily initialized documentation for all built-in words
static BUILTIN_DOCS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut docs = HashMap::new();

    // I/O Operations
    docs.insert(
        "io.write",
        "Write a string to stdout without a trailing newline.",
    );
    docs.insert(
        "io.write-line",
        "Write a string to stdout followed by a newline.",
    );
    docs.insert(
        "io.read-line",
        "Read a line from stdin. Returns (String Bool) -- Bool is false on EOF or read error.",
    );
    docs.insert(
        "io.read-line+",
        "DEPRECATED: Use io.read-line instead. Read a line from stdin. Returns (line, status_code).",
    );
    docs.insert(
        "io.read-n",
        "Read N bytes from stdin. Returns (bytes, status_code).",
    );

    // Command-line Arguments
    docs.insert("args.count", "Get the number of command-line arguments.");
    docs.insert("args.at", "Get the command-line argument at index N.");

    // File Operations
    docs.insert(
        "file.slurp",
        "Read entire file contents. Returns (String Bool) -- Bool is false if file not found or unreadable.",
    );
    docs.insert("file.exists?", "Check if a file exists at the given path.");
    docs.insert(
        "file.spit",
        "Write string to file (creates or overwrites). Returns Bool -- false on write failure.",
    );
    docs.insert(
        "file.append",
        "Append string to file (creates if needed). Returns Bool -- false on write failure.",
    );
    docs.insert(
        "file.delete",
        "Delete a file. Returns Bool -- false on failure.",
    );
    docs.insert(
        "file.size",
        "Get file size in bytes. Returns (Int Bool) -- Bool is false if file not found.",
    );
    docs.insert(
        "file.for-each-line+",
        "Execute a quotation for each line in a file.",
    );

    // Directory Operations
    docs.insert(
        "dir.exists?",
        "Check if a directory exists at the given path.",
    );
    docs.insert(
        "dir.make",
        "Create a directory (and parent directories if needed). Returns Bool -- false on failure.",
    );
    docs.insert(
        "dir.delete",
        "Delete an empty directory. Returns Bool -- false on failure.",
    );
    docs.insert(
        "dir.list",
        "List directory contents. Returns (List Bool) -- Bool is false if directory not found.",
    );

    // Type Conversions
    docs.insert(
        "int->string",
        "Convert an integer to its string representation.",
    );
    docs.insert(
        "int->float",
        "Convert an integer to a floating-point number.",
    );
    docs.insert("float->int", "Truncate a float to an integer.");
    docs.insert(
        "float->string",
        "Convert a float to its string representation.",
    );
    docs.insert(
        "string->int",
        "Parse a string as an integer. Returns (Int Bool) -- Bool is false if string is not a valid integer.",
    );
    docs.insert(
        "string->float",
        "Parse a string as a float. Returns (Float Bool) -- Bool is false if string is not a valid number.",
    );
    docs.insert(
        "char->string",
        "Convert a Unicode codepoint to a single-character string.",
    );
    docs.insert(
        "symbol->string",
        "Convert a symbol to its string representation.",
    );
    docs.insert("string->symbol", "Intern a string as a symbol.");

    // Integer Arithmetic
    docs.insert("i.add", "Add two integers.");
    docs.insert("i.subtract", "Subtract second integer from first.");
    docs.insert("i.multiply", "Multiply two integers.");
    docs.insert(
        "i.divide",
        "Integer division. Returns (result Bool) -- Bool is false on division by zero.",
    );
    docs.insert(
        "i.modulo",
        "Integer modulo. Returns (result Bool) -- Bool is false on division by zero.",
    );
    docs.insert("i.+", "Add two integers.");
    docs.insert("i.-", "Subtract second integer from first.");
    docs.insert("i.*", "Multiply two integers.");
    docs.insert(
        "i./",
        "Integer division. Returns (result Bool) -- Bool is false on division by zero.",
    );
    docs.insert(
        "i.%",
        "Integer modulo. Returns (result Bool) -- Bool is false on division by zero.",
    );

    // Integer Comparison
    docs.insert("i.=", "Test if two integers are equal.");
    docs.insert("i.<", "Test if first integer is less than second.");
    docs.insert("i.>", "Test if first integer is greater than second.");
    docs.insert(
        "i.<=",
        "Test if first integer is less than or equal to second.",
    );
    docs.insert(
        "i.>=",
        "Test if first integer is greater than or equal to second.",
    );
    docs.insert("i.<>", "Test if two integers are not equal.");
    docs.insert("i.eq", "Test if two integers are equal.");
    docs.insert("i.lt", "Test if first integer is less than second.");
    docs.insert("i.gt", "Test if first integer is greater than second.");
    docs.insert(
        "i.lte",
        "Test if first integer is less than or equal to second.",
    );
    docs.insert(
        "i.gte",
        "Test if first integer is greater than or equal to second.",
    );
    docs.insert("i.neq", "Test if two integers are not equal.");

    // Boolean Operations
    docs.insert("and", "Logical AND of two booleans.");
    docs.insert("or", "Logical OR of two booleans.");
    docs.insert("not", "Logical NOT of a boolean.");

    // Bitwise Operations
    docs.insert("band", "Bitwise AND of two integers.");
    docs.insert("bor", "Bitwise OR of two integers.");
    docs.insert("bxor", "Bitwise XOR of two integers.");
    docs.insert("bnot", "Bitwise NOT (complement) of an integer.");
    docs.insert("shl", "Shift left by N bits.");
    docs.insert("shr", "Shift right by N bits (arithmetic).");
    docs.insert(
        "i.neg",
        "Negate an integer (0 - n). Canonical name; `negate` is an alias.",
    );
    docs.insert(
        "negate",
        "Negate an integer (0 - n). Ergonomic alias for `i.neg`.",
    );
    docs.insert("popcount", "Count the number of set bits.");
    docs.insert("clz", "Count leading zeros.");
    docs.insert("ctz", "Count trailing zeros.");
    docs.insert("int-bits", "Push the bit width of integers (64).");

    // Stack Operations
    docs.insert("dup", "Duplicate the top stack value.");
    docs.insert("drop", "Remove the top stack value.");
    docs.insert("swap", "Swap the top two stack values.");
    docs.insert("over", "Copy the second value to the top.");
    docs.insert("rot", "Rotate the top three values (third to top).");
    docs.insert("nip", "Remove the second value from the stack.");
    docs.insert("tuck", "Copy the top value below the second.");
    docs.insert("2dup", "Duplicate the top two values.");
    docs.insert("3drop", "Remove the top three values.");
    docs.insert("pick", "Copy the value at depth N to the top.");
    docs.insert("roll", "Rotate N+1 items, bringing depth N to top.");

    // Aux Stack Operations
    docs.insert(
        ">aux",
        "Move top of stack to word-local aux stack. Must be balanced with aux> before word returns.",
    );
    docs.insert(
        "aux>",
        "Move top of aux stack back to main stack. Requires a matching >aux.",
    );

    // Channel Operations
    docs.insert(
        "chan.make",
        "Create a new channel for inter-strand communication.",
    );
    docs.insert(
        "chan.send",
        "Send a value on a channel. Returns Bool -- false if channel is closed.",
    );
    docs.insert(
        "chan.receive",
        "Receive a value from a channel. Returns (value Bool) -- Bool is false if channel is closed.",
    );
    docs.insert("chan.close", "Close a channel.");
    docs.insert("chan.yield", "Yield control to the scheduler.");

    // Control Flow
    docs.insert("call", "Call a quotation or closure.");
    docs.insert(
        "cond",
        "Multi-way conditional: test clauses until one succeeds.",
    );

    // Dataflow Combinators
    docs.insert(
        "dip",
        "Hide top value, run quotation on rest of stack, restore value. ( ..a x [..a -- ..b] -- ..b x )",
    );
    docs.insert(
        "keep",
        "Run quotation on top value, but preserve the original. ( ..a x [..a x -- ..b] -- ..b x )",
    );
    docs.insert(
        "bi",
        "Apply two quotations to the same value. ( ..a x [q1] [q2] -- ..c )",
    );

    // Concurrency
    docs.insert(
        "strand.spawn",
        "Spawn a concurrent strand. Returns strand ID.",
    );
    docs.insert(
        "strand.weave",
        "Create a generator/coroutine. Returns handle.",
    );
    docs.insert(
        "strand.resume",
        "Resume a weave with a value. Returns (handle, value, has_more).",
    );
    docs.insert(
        "yield",
        "Yield a value from a weave and receive resume value.",
    );
    docs.insert(
        "strand.weave-cancel",
        "Cancel a weave and release its resources.",
    );

    // TCP Operations
    docs.insert(
        "tcp.listen",
        "Start listening on a port. Returns (socket_id, success).",
    );
    docs.insert(
        "tcp.accept",
        "Accept a connection. Returns (client_id, success).",
    );
    docs.insert(
        "tcp.read",
        "Read data from a socket. Returns (string, success).",
    );
    docs.insert("tcp.write", "Write data to a socket. Returns success.");
    docs.insert("tcp.close", "Close a socket. Returns success.");

    // OS Operations
    docs.insert(
        "os.getenv",
        "Get environment variable. Returns (value, exists).",
    );
    docs.insert(
        "os.home-dir",
        "Get user's home directory. Returns (path, success).",
    );
    docs.insert(
        "os.current-dir",
        "Get current working directory. Returns (path, success).",
    );
    docs.insert("os.path-exists", "Check if a path exists.");
    docs.insert("os.path-is-file", "Check if path is a regular file.");
    docs.insert("os.path-is-dir", "Check if path is a directory.");
    docs.insert("os.path-join", "Join two path components.");
    docs.insert(
        "os.path-parent",
        "Get parent directory. Returns (path, success).",
    );
    docs.insert(
        "os.path-filename",
        "Get filename component. Returns (name, success).",
    );
    docs.insert("os.exit", "Exit the program with a status code.");
    docs.insert(
        "os.name",
        "Get the operating system name (e.g., \"macos\", \"linux\").",
    );
    docs.insert(
        "os.arch",
        "Get the CPU architecture (e.g., \"aarch64\", \"x86_64\").",
    );

    // Signal Handling
    docs.insert(
        "signal.trap",
        "Trap a signal: set internal flag on receipt instead of default action.",
    );
    docs.insert(
        "signal.received?",
        "Check if signal was received and clear the flag. Returns Bool.",
    );
    docs.insert(
        "signal.pending?",
        "Check if signal is pending without clearing the flag. Returns Bool.",
    );
    docs.insert(
        "signal.default",
        "Restore the default handler for a signal.",
    );
    docs.insert(
        "signal.ignore",
        "Ignore a signal entirely (useful for SIGPIPE in servers).",
    );
    docs.insert(
        "signal.clear",
        "Clear the pending flag for a signal without checking it.",
    );
    docs.insert("signal.SIGINT", "SIGINT constant (Ctrl+C interrupt).");
    docs.insert("signal.SIGTERM", "SIGTERM constant (termination request).");
    docs.insert("signal.SIGHUP", "SIGHUP constant (hangup detected).");
    docs.insert("signal.SIGPIPE", "SIGPIPE constant (broken pipe).");
    docs.insert(
        "signal.SIGUSR1",
        "SIGUSR1 constant (user-defined signal 1).",
    );
    docs.insert(
        "signal.SIGUSR2",
        "SIGUSR2 constant (user-defined signal 2).",
    );
    docs.insert("signal.SIGCHLD", "SIGCHLD constant (child status changed).");
    docs.insert("signal.SIGALRM", "SIGALRM constant (alarm clock).");
    docs.insert("signal.SIGCONT", "SIGCONT constant (continue if stopped).");

    // Terminal Operations
    docs.insert(
        "terminal.raw-mode",
        "Enable/disable raw terminal mode. In raw mode: no line buffering, no echo, Ctrl+C read as byte 3.",
    );
    docs.insert(
        "terminal.read-char",
        "Read a single byte from stdin (blocking). Returns 0-255 on success, -1 on EOF/error.",
    );
    docs.insert(
        "terminal.read-char?",
        "Read a single byte from stdin (non-blocking). Returns 0-255 if available, -1 otherwise.",
    );
    docs.insert(
        "terminal.width",
        "Get terminal width in columns. Returns 80 if unknown.",
    );
    docs.insert(
        "terminal.height",
        "Get terminal height in rows. Returns 24 if unknown.",
    );
    docs.insert(
        "terminal.flush",
        "Flush stdout. Use after writing escape sequences or partial lines.",
    );

    // String Operations
    docs.insert("string.concat", "Concatenate two strings.");
    docs.insert("string.length", "Get the character length of a string.");
    docs.insert("string.byte-length", "Get the byte length of a string.");
    docs.insert(
        "string.char-at",
        "Get Unicode codepoint at character index.",
    );
    docs.insert(
        "string.substring",
        "Extract substring from start index with length.",
    );
    docs.insert(
        "string.find",
        "Find substring. Returns index or -1 if not found.",
    );
    docs.insert("string.split", "Split string by delimiter. Returns a list.");
    docs.insert("string.contains", "Check if string contains a substring.");
    docs.insert(
        "string.starts-with",
        "Check if string starts with a prefix.",
    );
    docs.insert("string.empty?", "Check if string is empty.");
    docs.insert("string.equal?", "Check if two strings are equal.");
    docs.insert(
        "string.join",
        "Join a list of values with a separator string. ( list sep -- string )",
    );
    docs.insert("string.trim", "Remove leading and trailing whitespace.");
    docs.insert("string.chomp", "Remove trailing newline.");
    docs.insert("string.to-upper", "Convert to uppercase.");
    docs.insert("string.to-lower", "Convert to lowercase.");
    docs.insert("string.json-escape", "Escape special characters for JSON.");
    docs.insert("symbol.=", "Check if two symbols are equal.");

    // Encoding Operations
    docs.insert(
        "encoding.base64-encode",
        "Encode a string to Base64 (standard alphabet with padding).",
    );
    docs.insert(
        "encoding.base64-decode",
        "Decode a Base64 string. Returns (decoded, success).",
    );
    docs.insert(
        "encoding.base64url-encode",
        "Encode to URL-safe Base64 (no padding). Suitable for JWTs and URLs.",
    );
    docs.insert(
        "encoding.base64url-decode",
        "Decode URL-safe Base64. Returns (decoded, success).",
    );
    docs.insert(
        "encoding.hex-encode",
        "Encode a string to lowercase hexadecimal.",
    );
    docs.insert(
        "encoding.hex-decode",
        "Decode a hexadecimal string. Returns (decoded, success).",
    );

    // Crypto Operations
    docs.insert(
        "crypto.sha256",
        "Compute SHA-256 hash of a string. Returns 64-char hex digest.",
    );
    docs.insert(
        "crypto.hmac-sha256",
        "Compute HMAC-SHA256 signature. ( message key -- signature )",
    );
    docs.insert(
        "crypto.constant-time-eq",
        "Timing-safe string comparison. Use for comparing signatures/tokens.",
    );
    docs.insert(
        "crypto.random-bytes",
        "Generate N cryptographically secure random bytes as hex string.",
    );
    docs.insert(
        "crypto.random-int",
        "Generate uniform random integer in [min, max). ( min max -- Int ) Uses rejection sampling to avoid modulo bias.",
    );
    docs.insert("crypto.uuid4", "Generate a random UUID v4 string.");
    docs.insert(
        "crypto.aes-gcm-encrypt",
        "Encrypt with AES-256-GCM. ( plaintext hex-key -- ciphertext success )",
    );
    docs.insert(
        "crypto.aes-gcm-decrypt",
        "Decrypt AES-256-GCM ciphertext. ( ciphertext hex-key -- plaintext success )",
    );
    docs.insert(
        "crypto.pbkdf2-sha256",
        "Derive key from password. ( password salt iterations -- hex-key success ) Min 1000 iterations, 100000+ recommended.",
    );
    docs.insert(
        "crypto.ed25519-keypair",
        "Generate Ed25519 keypair. ( -- public-key private-key ) Both as 64-char hex strings.",
    );
    docs.insert(
        "crypto.ed25519-sign",
        "Sign message with Ed25519 private key. ( message private-key -- signature success ) Signature is 128-char hex.",
    );
    docs.insert(
        "crypto.ed25519-verify",
        "Verify Ed25519 signature. ( message signature public-key -- valid )",
    );

    // HTTP Client Operations
    docs.insert(
        "http.get",
        "HTTP GET request. ( url -- response-map ) Map has status, body, ok, error.",
    );
    docs.insert(
        "http.post",
        "HTTP POST request. ( url body content-type -- response-map )",
    );
    docs.insert(
        "http.put",
        "HTTP PUT request. ( url body content-type -- response-map )",
    );
    docs.insert(
        "http.delete",
        "HTTP DELETE request. ( url -- response-map )",
    );

    // Regular Expression Operations
    docs.insert(
        "regex.match?",
        "Check if pattern matches anywhere in string. ( text pattern -- bool )",
    );
    docs.insert(
        "regex.find",
        "Find first match. ( text pattern -- matched success )",
    );
    docs.insert(
        "regex.find-all",
        "Find all matches. ( text pattern -- list success )",
    );
    docs.insert(
        "regex.replace",
        "Replace first match. ( text pattern replacement -- result success )",
    );
    docs.insert(
        "regex.replace-all",
        "Replace all matches. ( text pattern replacement -- result success )",
    );
    docs.insert(
        "regex.captures",
        "Extract capture groups. ( text pattern -- groups success )",
    );
    docs.insert(
        "regex.split",
        "Split string by pattern. ( text pattern -- list success )",
    );
    docs.insert(
        "regex.valid?",
        "Check if pattern is valid regex. ( pattern -- bool )",
    );

    // Compression Operations
    docs.insert(
        "compress.gzip",
        "Compress string with gzip. Returns base64-encoded data. ( data -- compressed success )",
    );
    docs.insert(
        "compress.gzip-level",
        "Compress with gzip at level 1-9. ( data level -- compressed success )",
    );
    docs.insert(
        "compress.gunzip",
        "Decompress gzip data. ( base64-data -- decompressed success )",
    );
    docs.insert(
        "compress.zstd",
        "Compress string with zstd. Returns base64-encoded data. ( data -- compressed success )",
    );
    docs.insert(
        "compress.zstd-level",
        "Compress with zstd at level 1-22. ( data level -- compressed success )",
    );
    docs.insert(
        "compress.unzstd",
        "Decompress zstd data. ( base64-data -- decompressed success )",
    );

    // Variant Operations
    docs.insert(
        "variant.field-count",
        "Get the number of fields in a variant.",
    );
    docs.insert(
        "variant.tag",
        "Get the tag (constructor name) of a variant.",
    );
    docs.insert("variant.field-at", "Get the field at index N.");
    docs.insert(
        "variant.append",
        "Append a value to a variant (creates new).",
    );
    docs.insert("variant.last", "Get the last field of a variant.");
    docs.insert("variant.init", "Get all fields except the last.");
    docs.insert("variant.make-0", "Create a variant with 0 fields.");
    docs.insert("variant.make-1", "Create a variant with 1 field.");
    docs.insert("variant.make-2", "Create a variant with 2 fields.");
    docs.insert("variant.make-3", "Create a variant with 3 fields.");
    docs.insert("variant.make-4", "Create a variant with 4 fields.");
    docs.insert("variant.make-5", "Create a variant with 5 fields.");
    docs.insert("variant.make-6", "Create a variant with 6 fields.");
    docs.insert("variant.make-7", "Create a variant with 7 fields.");
    docs.insert("variant.make-8", "Create a variant with 8 fields.");
    docs.insert("variant.make-9", "Create a variant with 9 fields.");
    docs.insert("variant.make-10", "Create a variant with 10 fields.");
    docs.insert("variant.make-11", "Create a variant with 11 fields.");
    docs.insert("variant.make-12", "Create a variant with 12 fields.");
    docs.insert("wrap-0", "Create a variant with 0 fields (alias).");
    docs.insert("wrap-1", "Create a variant with 1 field (alias).");
    docs.insert("wrap-2", "Create a variant with 2 fields (alias).");
    docs.insert("wrap-3", "Create a variant with 3 fields (alias).");
    docs.insert("wrap-4", "Create a variant with 4 fields (alias).");
    docs.insert("wrap-5", "Create a variant with 5 fields (alias).");
    docs.insert("wrap-6", "Create a variant with 6 fields (alias).");
    docs.insert("wrap-7", "Create a variant with 7 fields (alias).");
    docs.insert("wrap-8", "Create a variant with 8 fields (alias).");
    docs.insert("wrap-9", "Create a variant with 9 fields (alias).");
    docs.insert("wrap-10", "Create a variant with 10 fields (alias).");
    docs.insert("wrap-11", "Create a variant with 11 fields (alias).");
    docs.insert("wrap-12", "Create a variant with 12 fields (alias).");

    // List Operations
    docs.insert("list.make", "Create an empty list.");
    docs.insert("list.push", "Push a value onto a list. Returns new list.");
    docs.insert(
        "list.get",
        "Get value at index. Returns (value Bool) -- Bool is false if index out of bounds.",
    );
    docs.insert(
        "list.set",
        "Set value at index. Returns (List Bool) -- Bool is false if index out of bounds.",
    );
    docs.insert("list.length", "Get the number of elements in a list.");
    docs.insert("list.empty?", "Check if a list is empty.");
    docs.insert("list.reverse", "Reverse the elements of a list.");
    docs.insert(
        "list.map",
        "Apply quotation to each element. Returns new list.",
    );
    docs.insert("list.filter", "Keep elements where quotation returns true.");
    docs.insert("list.fold", "Reduce list with accumulator and quotation.");
    docs.insert(
        "list.each",
        "Execute quotation for each element (side effects).",
    );

    // Map Operations
    docs.insert("map.make", "Create an empty map.");
    docs.insert(
        "map.get",
        "Get value for key. Returns (value Bool) -- Bool is false if key not found.",
    );
    docs.insert("map.set", "Set key to value. Returns new map.");
    docs.insert("map.has?", "Check if map contains a key.");
    docs.insert("map.remove", "Remove a key. Returns new map.");
    docs.insert("map.keys", "Get all keys as a list.");
    docs.insert("map.values", "Get all values as a list.");
    docs.insert("map.size", "Get the number of key-value pairs.");
    docs.insert("map.empty?", "Check if map is empty.");
    docs.insert(
        "map.each",
        "Iterate key-value pairs. Quotation: ( key value -- ).",
    );
    docs.insert(
        "map.fold",
        "Fold over key-value pairs with accumulator. Quotation: ( acc key value -- acc' ).",
    );

    // TCP Operations
    docs.insert(
        "tcp.listen",
        "Start listening on a port. Returns (fd Bool) -- Bool is false on failure.",
    );
    docs.insert(
        "tcp.accept",
        "Accept a connection. Returns (fd Bool) -- Bool is false on failure.",
    );
    docs.insert(
        "tcp.read",
        "Read from a connection. Returns (String Bool) -- Bool is false on failure.",
    );
    docs.insert(
        "tcp.write",
        "Write to a connection. Returns Bool -- false on failure.",
    );
    docs.insert(
        "tcp.close",
        "Close a connection. Returns Bool -- false on failure.",
    );

    // OS Operations
    docs.insert(
        "os.getenv",
        "Get environment variable. Returns (String Bool) -- Bool is false if not set.",
    );
    docs.insert(
        "os.home-dir",
        "Get home directory. Returns (String Bool) -- Bool is false if unavailable.",
    );
    docs.insert(
        "os.current-dir",
        "Get current directory. Returns (String Bool) -- Bool is false if unavailable.",
    );
    docs.insert("os.path-exists", "Check if a path exists.");
    docs.insert("os.path-is-file", "Check if a path is a file.");
    docs.insert("os.path-is-dir", "Check if a path is a directory.");
    docs.insert("os.path-join", "Join two path segments.");
    docs.insert(
        "os.path-parent",
        "Get parent directory. Returns (String Bool) -- Bool is false for root.",
    );
    docs.insert(
        "os.path-filename",
        "Get filename component. Returns (String Bool) -- Bool is false if none.",
    );
    docs.insert("os.exit", "Exit the process with given exit code.");
    docs.insert("os.name", "Get OS name (e.g., \"macos\", \"linux\").");
    docs.insert(
        "os.arch",
        "Get CPU architecture (e.g., \"aarch64\", \"x86_64\").",
    );

    // Regex Operations
    docs.insert("regex.match?", "Test if string matches regex pattern.");
    docs.insert(
        "regex.find",
        "Find first match. Returns (String Bool) -- Bool is false if no match or invalid regex.",
    );
    docs.insert(
        "regex.find-all",
        "Find all matches. Returns (List Bool) -- Bool is false if invalid regex.",
    );
    docs.insert(
        "regex.replace",
        "Replace first match. Returns (String Bool) -- Bool is false if invalid regex.",
    );
    docs.insert(
        "regex.replace-all",
        "Replace all matches. Returns (String Bool) -- Bool is false if invalid regex.",
    );
    docs.insert(
        "regex.captures",
        "Get capture groups. Returns (List Bool) -- Bool is false if invalid regex.",
    );
    docs.insert(
        "regex.split",
        "Split by regex. Returns (List Bool) -- Bool is false if invalid regex.",
    );
    docs.insert("regex.valid?", "Check if a regex pattern is valid.");

    // Encoding Operations
    docs.insert("encoding.base64-encode", "Encode string as base64.");
    docs.insert(
        "encoding.base64-decode",
        "Decode base64 string. Returns (String Bool) -- Bool is false if invalid.",
    );
    docs.insert("encoding.base64url-encode", "Encode string as base64url.");
    docs.insert(
        "encoding.base64url-decode",
        "Decode base64url string. Returns (String Bool) -- Bool is false if invalid.",
    );
    docs.insert("encoding.hex-encode", "Encode string as hexadecimal.");
    docs.insert(
        "encoding.hex-decode",
        "Decode hex string. Returns (String Bool) -- Bool is false if invalid.",
    );

    // Crypto Operations
    docs.insert("crypto.sha256", "Compute SHA-256 hash of a string.");
    docs.insert(
        "crypto.hmac-sha256",
        "Compute HMAC-SHA256. ( message key -- hash )",
    );
    docs.insert(
        "crypto.constant-time-eq",
        "Constant-time string equality comparison.",
    );
    docs.insert(
        "crypto.random-bytes",
        "Generate N random bytes as a string.",
    );
    docs.insert(
        "crypto.random-int",
        "Generate random integer in range [min, max).",
    );
    docs.insert("crypto.uuid4", "Generate a random UUID v4 string.");
    docs.insert(
        "crypto.aes-gcm-encrypt",
        "AES-GCM encrypt. Returns (String Bool) -- Bool is false on failure.",
    );
    docs.insert("crypto.aes-gcm-decrypt", "AES-GCM decrypt. Returns (String Bool) -- Bool is false on failure (wrong key or tampered data).");
    docs.insert(
        "crypto.pbkdf2-sha256",
        "Derive key with PBKDF2. Returns (String Bool) -- Bool is false on failure.",
    );
    docs.insert(
        "crypto.ed25519-keypair",
        "Generate Ed25519 keypair. Returns (public private).",
    );
    docs.insert(
        "crypto.ed25519-sign",
        "Sign with Ed25519. Returns (String Bool) -- Bool is false on failure.",
    );
    docs.insert(
        "crypto.ed25519-verify",
        "Verify Ed25519 signature. Returns Bool -- true if valid.",
    );

    // Compression Operations
    docs.insert(
        "compress.gzip",
        "Gzip compress. Returns (String Bool) -- Bool is false on failure.",
    );
    docs.insert(
        "compress.gzip-level",
        "Gzip compress at level N. Returns (String Bool) -- Bool is false on failure.",
    );
    docs.insert(
        "compress.gunzip",
        "Gzip decompress. Returns (String Bool) -- Bool is false on failure.",
    );
    docs.insert(
        "compress.zstd",
        "Zstd compress. Returns (String Bool) -- Bool is false on failure.",
    );
    docs.insert(
        "compress.zstd-level",
        "Zstd compress at level N. Returns (String Bool) -- Bool is false on failure.",
    );
    docs.insert(
        "compress.unzstd",
        "Zstd decompress. Returns (String Bool) -- Bool is false on failure.",
    );

    // Signal Operations
    docs.insert(
        "signal.trap",
        "Register a signal handler for the given signal number.",
    );
    docs.insert("signal.received?", "Check if a signal has been received.");
    docs.insert("signal.pending?", "Check if a signal is pending.");
    docs.insert("signal.default", "Reset signal to default handler.");
    docs.insert("signal.ignore", "Ignore a signal.");
    docs.insert("signal.clear", "Clear pending signal.");

    // Terminal Operations
    docs.insert("terminal.raw-mode", "Enable/disable raw terminal mode.");
    docs.insert("terminal.read-char", "Read a single character (blocking).");
    docs.insert(
        "terminal.read-char?",
        "Read a character if available (non-blocking). Returns 0 if none.",
    );
    docs.insert("terminal.width", "Get terminal width in columns.");
    docs.insert("terminal.height", "Get terminal height in rows.");
    docs.insert("terminal.flush", "Flush stdout.");

    // Float Arithmetic
    docs.insert("f.add", "Add two floats.");
    docs.insert("f.subtract", "Subtract second float from first.");
    docs.insert("f.multiply", "Multiply two floats.");
    docs.insert("f.divide", "Divide first float by second.");
    docs.insert("f.+", "Add two floats.");
    docs.insert("f.-", "Subtract second float from first.");
    docs.insert("f.*", "Multiply two floats.");
    docs.insert("f./", "Divide first float by second.");

    // Float Comparison
    docs.insert("f.=", "Test if two floats are equal.");
    docs.insert("f.<", "Test if first float is less than second.");
    docs.insert("f.>", "Test if first float is greater than second.");
    docs.insert("f.<=", "Test if first float is less than or equal.");
    docs.insert("f.>=", "Test if first float is greater than or equal.");
    docs.insert("f.<>", "Test if two floats are not equal.");
    docs.insert("f.eq", "Test if two floats are equal.");
    docs.insert("f.lt", "Test if first float is less than second.");
    docs.insert("f.gt", "Test if first float is greater than second.");
    docs.insert("f.lte", "Test if first float is less than or equal.");
    docs.insert("f.gte", "Test if first float is greater than or equal.");
    docs.insert("f.neq", "Test if two floats are not equal.");

    // Test Framework
    docs.insert(
        "test.init",
        "Initialize the test framework with a test name.",
    );
    docs.insert("test.finish", "Finish testing and print results.");
    docs.insert("test.has-failures", "Check if any tests have failed.");
    docs.insert("test.assert", "Assert that a boolean is true.");
    docs.insert("test.assert-not", "Assert that a boolean is false.");
    docs.insert("test.assert-eq", "Assert that two integers are equal.");
    docs.insert("test.assert-eq-str", "Assert that two strings are equal.");
    docs.insert("test.fail", "Mark a test as failed with a message.");
    docs.insert("test.pass-count", "Get the number of passed assertions.");
    docs.insert("test.fail-count", "Get the number of failed assertions.");

    // Time Operations
    docs.insert("time.now", "Get current Unix timestamp in seconds.");
    docs.insert(
        "time.nanos",
        "Get high-resolution monotonic time in nanoseconds.",
    );
    docs.insert("time.sleep-ms", "Sleep for N milliseconds.");

    // Serialization
    docs.insert("son.dump", "Serialize any value to SON format (compact).");
    docs.insert(
        "son.dump-pretty",
        "Serialize any value to SON format (pretty-printed).",
    );

    // Stack Introspection
    docs.insert(
        "stack.dump",
        "Print all stack values and clear the stack (REPL).",
    );

    docs
});
