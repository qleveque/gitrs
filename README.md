# gitrs

> A fast, intuitive Git TUI written in Rust with [ratatui](https://github.com/ratatui-org/ratatui), heavily inspired by [tig](https://github.com/jonas/tig).

![demo](https://github.com/qleveque/gitrs/blob/main/resources/demo.gif?raw=true)

---

## Features

* Status, Log, Show, Diff, Blame, and Stash views
* Interactive Git pager with smooth navigation
* Fully customizable key mappings and mouse-friendly buttons
* Built-in and shell-integrated actions
* Asynchronous command execution
* Minimalist, fast, and intuitive interface

Each view reimagines a common Git workflow, making it faster, simpler, and more accessible—whether you're using the keyboard or clicking through with the mouse.

---

## Usage

```bash
gitrs status
gitrs show [revision]
gitrs blame <file> [line]
gitrs stash
gitrs log [...params]
gitrs diff [...params]
git config --global core.pager gitrs
```

Once started, you can navigate using the:
* __Mouse__: left and right clicks, you can also use the menu bar buttons.
* __Keyboard__: arrow keys, <kbd>Enter</kbd>, <kbd>Ctrl</kbd><kbd>F</kbd>, <kbd>Escape</kbd> and familiar shortcuts for navigation and interaction.

---

## Advanced Usage

gitrs is initially designed to be fully __keyboard-driven__ and __highly customizable__. It comes with a built-in default configuration that's applied automatically.

See [KEYBINDINGS.md](KEYBINDINGS.md) for a full list of key mappings.

See the [default configuration](./config/.gitrsrc) to explore or customize it.

---

## Configuration

Configure gitrs by creating a `~/.gitrsrc` file.
See the [default configuration](./config/.gitrsrc) for examples.

```bash
# Map Hotkeys
map <scope> <keys> <action>
# Create a button
button <scope> <text> <action>
# Set an option
set <option> <value>
```

### Actions

By default, actions can be run at runtime by pressing <kbd>:</kbd> and typing the desired one. An action can be a:

- **Shell command**:
    * `!` Run and wait
    * `>` Run, then exit
    * `@` Run asynchronously
    * Placeholders:
        * `%(rev)` will be replaced by the current commit hash
        * `%(file)` by the current file path
        * `%(line)` by the current context line
        * `%(text)` by the current line text
        * `%(git)` by the git executable
        * `%(clip)` by the clipboard utility
- **Builtin command**:
    - Navigation: `up`, `down`, `first`, `last`, `shift_line_middle`, `shift_line_top`, `shift_line_bottom`
    - Go to specific line: `goto [line]`, `:<line>`
    - Config: `map <scope> <keys> <action>`, `button <scope> <text> <action>`, `set <option> <value>`
    - Search: `search`, `search_reverse`, `next_search_result`, `previous_search_result`
    - Status specific: `status_switch_view`, `stage_unstage_file`, `stage_unstage_files`
    - Blame specific: `next_commit_blame`, `previous_commit_blame`
    - Log specific: `pager_next_commit`, `pager_previous_commit`
    - Stash specific: `stash_drop`, `stash_apply`, `stash_pop`
    - Others: `nop`, `echo`, `reload`, `quit`, `open_show_app`, `open_git_show`, `open_log_app`

### Scopes

* `global`
* `show[:(new|modified|deleted)]`
* `status[:(staged|unstaged)[:(new|modified|deleted|conflicted)]]`
* `log` `diff` `pager`
* `blame`
* `stash`

### Options

| Option | Description | Default | Type |
|:---|:---|:---|:---|
| `git` | Path to Git executable (useful for WSL: `git.exe`) | `"git"` | string |
| `clipboard` | Clipboard utility to use | `"clip.exe"` on Windows and `"xsel"` on Linux | string |
| `scrolloff` | Number of lines to keep above/below cursor | `5` | usize |
| `scroll_step` | Number of lines per scroll step | `2` | `usize` |
| `smart_case` | Use smart case | `true` | `false \| true` |
| `menu_bar` | Show the menu bar | `true` | `false \| true` |
| `default_mappings` | Load the default mappings | `true` | `false \| true` |
| `default_buttons` | Load the default buttons | `true` | `false \| true` |

---

## Contributing

Contributions are welcome!  
Feel free to open issues, suggest improvements, or submit pull requests.

---

## License

This project is licensed under the [MIT License](LICENSE).
