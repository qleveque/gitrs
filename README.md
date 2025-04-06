# gitrs

A git TUI written in Rust with ratatui.

![demo](https://github.com/qleveque/gitrs/blob/main/resources/demo.gif?raw=true)

This is still a work in progress.

## Usage

+ <kbd>k</kbd>/<kbd>↑</kbd> Go up
+ <kbd>j</kbd>/<kbd>↓</kbd> Go down
+ <kbd>g</kbd>/<kbd>Home</kbd> Go to first
+ <kbd>G</kbd>/<kbd>End</kbd> Go to last
+ <kbd>q</kbd>/<kbd>Backspace</kbd> Go back
+ <kbd>Ctrl+u</kbd> Go up half a page
+ <kbd>Ctrl+d</kbd> Go down half a page

## Commands

### `$ gitrs status`

List new/modified/deleted files from the working tree and the staging area.

+ <kbd>t</kbd> Stage/unstage selected file
+ <kbd>T</kbd> Stage/unstage all the files
+ <kbd>Tab</kbd> Switch between untracked and staged column
+ <kbd>J</kbd> Select staged column
+ <kbd>K</kbd> Select untracked column
+ <kbd>r</kbd> Reload the view

### `$ gitrs show [revision]`

Show commit details: hash, references, author, date, commit title and message.
Display the list of modified files.

### `$ gitrs blame <file> [line]`

Show the blame of the given file.
+ <kbd>Enter</kbd> Show commit defails
+ <kbd>h</kbd>/<kbd>←</kbd> Go to parent blame
+ <kbd>l</kbd>/<kbd>→</kbd> Go back to previous blame

## Configure

You can configure the behaviour of gitrs through the `~/.gitrsrc` file.

You can create command shorcuts as follow:
```
map <scope> <hotkey> [>!@]<command>
```
`scope` can be either of `global`, `blame`, `show`, `status`, `staged`, `unstaged`, `unmerged`, `untracked`.

+ `!`: run and wait for the command to execute
+ `>`: like `!` but it exits gitrs after the command execution
+ `@`: run the command asynchronously in the background

You can also configure other parameters as follow:
```
set {key} {value}
```

+ `git`: set the git executable (useful if on WSL and you want to use `git.exe`)
+ `scrolloff`: configure the scroll off

## Configuration example (`~/.gitrsrc`)

```
map global d !git difftool '%(rev)^..%(rev)' '%(file)'
map staged d !git difftool --staged '%(file)'
map status C >git commit
map status A >git commit --amend
map status N >git commit --amend --no-edit
map untracked D !rm "%(file)"
map unstaged ! !git restore "%(file)"
map global yf @echo -n %(file) | xsel --input --clipboard
map global yc @echo -n %(rev) | xsel --input --clipboard
```

## TODO

- [ ] Improve the code quality
- [ ] Searchbar to search for a specific content
- [ ] Add the `log` command
- [ ] Add the `stash` command
- [ ] Add the `reflog` command
- [ ] Add the `branch` command
- [x] Allow multiple keystrokes in a command hotkey
- [ ] Allow the use of modifiers (ctrl/shift) in a command hotkey
- [ ] Handle renames

## Credits
This tool was inspired by [tig](https://github.com/jonas/tig).
