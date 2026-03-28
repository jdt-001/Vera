//! `vera overview` — Show architecture overview of the indexed project.

use vera_core::stats::{self, ProjectOverview};

/// Run the `vera overview` command.
pub fn run(json_output: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| anyhow::anyhow!("failed to get current directory: {e}"))?;

    let overview = stats::collect_overview(&cwd)?;

    if json_output {
        let json = serde_json::to_string_pretty(&overview)
            .map_err(|e| anyhow::anyhow!("failed to serialize overview: {e}"))?;
        println!("{json}");
    } else {
        print_human_overview(&overview);
    }

    Ok(())
}

fn print_human_overview(o: &ProjectOverview) {
    println!("Project Overview");
    println!();
    println!(
        "  Files: {}    Chunks: {}    Lines: ~{}    Index size: {}",
        o.file_count, o.chunk_count, o.total_lines, o.index_size_human
    );

    if !o.languages.is_empty() {
        println!();
        println!("  Languages:");
        for lang in &o.languages {
            println!(
                "    {:<15} {:>4} files, {:>5} chunks",
                lang.language, lang.files, lang.chunks
            );
        }
    }

    if !o.top_directories.is_empty() {
        println!();
        println!("  Top Directories:");
        for dir in &o.top_directories {
            println!("    {:<30} {:>4} files", dir.directory, dir.files);
        }
    }

    if !o.symbol_types.is_empty() {
        println!();
        println!("  Symbol Types:");
        for st in &o.symbol_types {
            println!("    {:<15} {:>5}", st.symbol_type, st.count);
        }
    }

    if !o.entry_points.is_empty() {
        println!();
        println!("  Entry Points:");
        for ep in &o.entry_points {
            println!("    {ep}");
        }
    }

    if !o.hotspots.is_empty() {
        println!();
        println!("  Hotspots (most complex files):");
        for h in &o.hotspots {
            println!("    {:<50} {:>3} chunks", h.file_path, h.chunks);
        }
    }
}
