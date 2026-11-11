//! macOS project selection before the editor event loop starts.
use std::{error::Error, path::PathBuf, process::Command};

const PICK: &str = r#"
on run argv
    try
        set initialFolder to POSIX file (item 2 of argv) as alias
        set action to button returned of (display dialog (item 1 of argv) with title "Voxy Projects" buttons {"Cancel", "Open Project", "New Project"} default button "New Project" cancel button "Cancel")
        if action is "Open Project" then
            set descriptor to choose file with prompt "Choose a Voxy project file" default location initialFolder
            return "open" & linefeed & POSIX path of descriptor
        end if
        set parentFolder to choose folder with prompt "Choose where to create the project" default location initialFolder
        set projectName to text returned of (display dialog "Project name and folder name" with title "New Voxy Project" default answer "" buttons {"Cancel", "Create"} default button "Create" cancel button "Cancel")
        if projectName contains linefeed or projectName contains return then return "invalid-name"
        return "create" & linefeed & POSIX path of parentFolder & linefeed & projectName
    on error number -128
        return "cancel"
    end try
end run
"#;

pub(crate) fn choose() -> Result<Option<PathBuf>, Box<dyn Error>> {
    let mut prompt = String::from("Create a project or open an existing one.");
    let initial_folder = std::env::current_dir()?;
    loop {
        let result = Command::new("/usr/bin/osascript")
            .args(["-e", PICK, "--", &prompt])
            .arg(&initial_folder)
            .output()?;
        if !result.status.success() {
            return Err(String::from_utf8_lossy(&result.stderr)
                .trim()
                .to_owned()
                .into());
        }
        let reply = String::from_utf8(result.stdout)?;
        let reply = reply
            .strip_suffix('\n')
            .ok_or("incomplete project picker reply")?;
        if reply == "cancel" {
            return Ok(None);
        }
        if reply == "invalid-name" {
            prompt = "Enter a single-line project name.".into();
            continue;
        }
        let (action, value) = reply
            .split_once('\n')
            .ok_or("missing project picker selection")?;
        let picked = match action {
            "open" => {
                let path = PathBuf::from(value);
                let registry = voxy_editor::editor_component_registry()?;
                voxy_editor::project::open_for_editor(&path, &registry).map(|_| path)
            }
            "create" => {
                let (parent, name) = value.rsplit_once('\n').ok_or("missing project name")?;
                let name = name.trim();
                if name.is_empty() || name == "." || name == ".." || name.contains(['/', ':', '\r'])
                {
                    prompt = "Enter a project name that can be used as a folder name.".into();
                    continue;
                }
                voxy_editor::project::create(&PathBuf::from(parent).join(name), name)
            }
            _ => return Err("unknown project picker selection".into()),
        };
        match picked {
            Ok(path) => return Ok(Some(path)),
            Err(error) => prompt = error.to_string(),
        }
    }
}
