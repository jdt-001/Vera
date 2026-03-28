//! CLI handlers for `vera references` and `vera dead-code`.

use anyhow::Result;

/// Run the `vera references <symbol>` command.
pub fn run(symbol: &str, callees: bool, json: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;

    if callees {
        let results = vera_core::stats::find_callees(&cwd, symbol)?;
        if json {
            println!("{}", serde_json::to_string(&results)?);
        } else if results.is_empty() {
            println!("No callees found for '{symbol}'.");
        } else {
            println!(
                "Symbols called by '{symbol}' ({} results):\n",
                results.len()
            );
            for r in &results {
                println!("  {}:{} → {}", r.file_path, r.line, r.callee);
            }
        }
    } else {
        let results = vera_core::stats::find_callers(&cwd, symbol)?;
        if json {
            println!("{}", serde_json::to_string(&results)?);
        } else if results.is_empty() {
            println!("No callers found for '{symbol}'.");
        } else {
            println!("Callers of '{symbol}' ({} results):\n", results.len());
            for r in &results {
                let caller = r.caller.as_deref().unwrap_or("<top-level>");
                println!("  {}:{} in {}", r.file_path, r.line, caller);
            }
        }
    }
    Ok(())
}

/// Run the `vera dead-code` command.
pub fn run_dead_code(json: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let results = vera_core::stats::find_dead_symbols(&cwd)?;

    if json {
        println!("{}", serde_json::to_string(&results)?);
    } else if results.is_empty() {
        println!("No dead code found.");
    } else {
        println!("Potentially unused symbols ({} results):\n", results.len());
        for r in &results {
            let stype = r.symbol_type.as_deref().unwrap_or("symbol");
            println!("  {}:{} {} {}", r.file_path, r.line, stype, r.symbol_name);
        }
    }
    Ok(())
}
