# gitrs

> A fast, intuitive Git TUI written in Rust with [ratatui](https://github.com/ratatui-org/ratatui), heavily inspired by [tig](https://github.com/jonas/tig).

![demo](https://github.com/qleveque/gitrs/blob/main/resources/demo.gif?raw=true)

---

## Table of Contents

- [Features](#features)
- [Usage](#usage)
- [Default Key Bindings](#default-key-bindings)
- [Actions](#actions)
- [Configuration](#configuration)
- [Contributing](#contributing)
- [License](#license)

---

## Features

- Status, Log, Show, and Blame views
- Highly customizable key mappings
- Built-in and shell action support
- Asynchronous command execution
- Minimal, fast, and intuitive interface

Each view reimagines a common Git workflow to make it faster, simpler, and more user-friendly than traditional Git commands.

---

## Usage

```bash
gitrs status
gitrs log [...params]
gitrs show [revision]
gitrs blame <file> [line]
```

---

## Default Key Bindings

<details>
<summary><strong>Global</strong></summary>

| Key | Action |
|:---|:---|
| <kbd>k</kbd> / <kbd>↑</kbd> | Go up |
| <kbd>j</kbd> / <kbd>↓</kbd> | Go down |
| <kbd>g</kbd><kbd>g</kbd> / <kbd>Home</kbd> | Go to first item |
| <kbd>G</kbd> / <kbd>End</kbd> | Go to last item |
| <kbd>Ctrl+u</kbd> / <kbd>PageUp</kbd> | Half-page up |
| <kbd>Ctrl+d</kbd> / <kbd>PageDown</kbd> | Half-page down |
| <kbd>r</kbd> | Reload |
| <kbd>q</kbd> / <kbd>Esc</kbd> | Quit |
| <kbd>/</kbd> / <kbd>Ctrl+f</kbd> | Search forward |
| <kbd>?</kbd> | Search backward |
| <kbd>n</kbd> | Next search result |
| <kbd>N</kbd> | Previous search result |
| <kbd>z</kbd><kbd>z</kbd> | Center current line |
| <kbd>z</kbd><kbd>t</kbd> | Move current line to top |
| <kbd>z</kbd><kbd>b</kbd> | Move current line to bottom |
| <kbd>y</kbd><kbd>c</kbd> | Copy current commit hash to clipboard |
| <kbd>y</kbd><kbd>f</kbd> | Copy current filename to clipboard |
| <kbd>y</kbd><kbd>y</kbd> | Copy current text line to clipboard |
| <kbd>:</kbd> | Type and run an [action](#actions) |

</details>
<details>
<summary><strong>Status</strong></summary>

| Key | Action |
|:---|:---|
| <kbd>Enter</kbd> | Open `git difftool` |
| <kbd>t</kbd> / <kbd>Space</kbd> | Stage/unstage selected file |
| <kbd>T</kbd> | Stage/unstage all files |
| <kbd>Tab</kbd> | Switch between columns |
| <kbd>J</kbd> | Focus staged files |
| <kbd>K</kbd> | Focus unstaged/untracked files |
| <kbd>!</kbd><kbd>c</kbd> | `git commit` |
| <kbd>!</kbd><kbd>a</kbd> | `git commit --amend` |
| <kbd>!</kbd><kbd>n</kbd> | `git commit --amend --no-edit` |
| <kbd>!</kbd><kbd>p</kbd> | `git push` |
| <kbd>!</kbd><kbd>P</kbd> | `git push --force` |
| <kbd>!</kbd><kbd>r</kbd> | Restore modified files or remove untracked files |

</details>
<details>
<summary><strong>Log</strong></summary>

| Key | Action |
|:---|:---|
| <kbd>Enter</kbd> | Show commit details |
| <kbd>c</kbd> | Next commit |
| <kbd>C</kbd> | Previous commit |
| <kbd>d</kbd> | Open current patch with `git difftool` |

</details>
<details>
<summary><strong>Show</strong></summary>

| Key | Action |
|:---|:---|
| <kbd>Enter</kbd> | Open `git difftool` |

</details>
<details>
<summary><strong>Blame</strong></summary>

| Key | Action |
|:---|:---|
| <kbd>Enter</kbd> | Show commit details |
| <kbd>h</kbd> / <kbd>←</kbd> | Go to parent blame |
| <kbd>l</kbd> / <kbd>→</kbd> | Return to previous blame |

</details>

---

## Actions

An action can be a:

- **Shell command**:
    * `!` Run and wait
    * `>` Run, then exit
    * `@` Run asynchronously
    * Placeholders:
        * `%(rev)` will be replaced by the current commit hash
        * `%(file)` will be replaced by the current file path
        * `%(line)` will be replaced by the current context line
        * `%(text)` will be replaced by the current line text
- **Builtin command**:
    - Navigation: `up`, `down`, `first`, `last`, `shift_line_middle`, `shift_line_top`, `shift_line_bottom`
    - Search: `search`, `search_reverse`, `next_search_result`, `previous_search_result`
    - Status specific: `stage_unstage_file`, `stage_unstage_files`, `show_commit`
    - Blame specific: `next_commit_blame`, `previous_commit_blame`
    - Log specific: `next_commit`, `previous_commit`
    - Others: `nop`, `reload`, `quit`

---

## Configuration

Configure gitrs by creating a `~/.gitrsrc` file.

#### Mapping Hotkeys

```bash
map <scope> <keys> <action>
```
* scope: `global` | `show` | `status` | `unstaged` | `staged` | `unmerged` | `untracked` | `log` | `blame`
* keys: a vim-like key binding sequence (e.g. `<c-x>E`)

#### Setting Options

```bash
set <option> <value>
```
| Option | Description | Default | Type |
|:---|:---|:---|:---|
| `git` | Path to Git executable (useful for WSL: `git.exe`) | `"git"` | string |
| `clipboard` | Clipboard utility to use | `"clip.exe"` on Windows and `"xsel"` on Linux | string |
| `scrolloff` | Number of lines to keep above/below cursor | `5` | usize |
| `smartcase` | Use smart case or not | `"true"` | `"false" \| "true"` |

---

## Contributing

Contributions are welcome!  
Feel free to open issues, suggest improvements, or submit pull requests.

---

## License

This project is licensed under the [MIT License](LICENSE).
