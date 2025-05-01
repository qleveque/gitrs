# gitrs

> A fast, intuitive Git TUI written in Rust with [ratatui](https://github.com/ratatui-org/ratatui), heavily inspired by [tig](https://github.com/jonas/tig).

![demo](https://github.com/qleveque/gitrs/blob/main/resources/demo.gif?raw=true)

---

## Table of Contents

- [Features](#features)
- [Usage](#usage)
- [Git Pager](#git-pager)
- [Default Key Bindings](#default-key-bindings)
- [Actions](#actions)
- [Configuration](#configuration)
- [Contributing](#contributing)
- [License](#license)

---

## Features

* Status, Log, Show, Reflog, Files, Blame, and Stash views
* Acts as a pager for Git with interactive browsing
* Highly customizable key mappings
* Built-in and shell action support
* Asynchronous command execution
* Minimal, fast, and intuitive interface

Each view reimagines a common Git workflow to make it faster, simpler, and more user-friendly than traditional Git commands.

---

## Usage

```bash
gitrs status
gitrs log [...params]
gitrs show [...params]
gitrs reflog [...params]
gitrs stash
gitrs files [revision]
gitrs blame <file> [line]
git config --global core.pager "gitrs"
```

Once started, you can navigate using:
* __Mouse__: left and right clicks, the menu bar buttons.
* __Keyboard__: arrow keys, <kbd>Enter</kbd>, <kbd>Ctrl</kbd><kbd>F</kbd>, <kbd>Escape</kbd> and familiar shortcuts for seamless navigation and interaction

---

## Advanced Usage

gitrs is built to be __keyboard-driven__ and __highly customizable__. It comes with a built-in default configuration that's applied automatically.
See the [default configuration](./config/.gitrsrc) to explore or customize it.

---

## Actions

An action can be a:

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
    - Search: `search`, `search_reverse`, `next_search_result`, `previous_search_result`
    - Status specific: `status_switch_view`, `stage_unstage_file`, `stage_unstage_files`
    - Blame specific: `next_commit_blame`, `previous_commit_blame`
    - Log specific: `pager_next_commit`, `pager_previous_commit`
    - Stash specific: `stash_drop`, `stash_apply`, `stash_pop`
    - Others: `nop`, `reload`, `quit`, `open_files_app`, `open_show_app`

---

## Configuration

Configure gitrs by creating a `~/.gitrsrc` file.
See the [default configuration](./config/.gitrsrc) for more examples.

#### Mapping Hotkeys

```bash
map <scope> <keys> <action>
```
where keys is a vim-like key binding sequence.

#### Adding a button

```bash
map <scope> <text> <action>
```

#### Scopes

* `global`
* `files`
* `status` `unstaged` `staged` `unmerged` `untracked`
* `pager` `show` `log` `reflog`
* `blame`
* `stash`

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
| `scrollstep` | Configure number of lines per scroll step | `2` | `usize` |
| `menubar` | Whether or not the menu bar is shown | `"true"` | `"false" \| "true"` |

---

## Contributing

Contributions are welcome!  
Feel free to open issues, suggest improvements, or submit pull requests.

---

## License

This project is licensed under the [MIT License](LICENSE).
