//! Embedded skill files that `vera agent install` writes to agent skill dirs.

pub struct SkillFile {
    pub relative_path: &'static str,
    pub contents: &'static str,
}

pub const VERA_SKILL_NAME: &str = "vera";

pub const VERA_SKILL_FILES: &[SkillFile] = &[
    SkillFile {
        relative_path: "SKILL.md",
        contents: include_str!("../../../skills/vera/SKILL.md"),
    },
    SkillFile {
        relative_path: "references/install.md",
        contents: include_str!("../../../skills/vera/references/install.md"),
    },
    SkillFile {
        relative_path: "references/query-patterns.md",
        contents: include_str!("../../../skills/vera/references/query-patterns.md"),
    },
    SkillFile {
        relative_path: "references/troubleshooting.md",
        contents: include_str!("../../../skills/vera/references/troubleshooting.md"),
    },
    SkillFile {
        relative_path: "references/mcp.md",
        contents: include_str!("../../../skills/vera/references/mcp.md"),
    },
    SkillFile {
        relative_path: "agents/openai.yaml",
        contents: include_str!("../../../skills/vera/agents/openai.yaml"),
    },
];
