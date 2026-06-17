# TermTab

TermTab is a lightning-fast, ergonomically-focused Vim-style terminal user interface for creating and editing guitar tabs. Built in Rust, it uses a 1D column-stream architecture that seamlessly wraps around your screen, providing a fluid text-editor-like experience for musical notation.

## Features

- **Vim-Style Navigation**: Instantly navigate your tabs using `h`, `j`, `k`, `l`, or jump measure-by-measure using `w`, `e`, and `b`. Numeric prefixes are fully supported (e.g. type `5l` to jump 5 columns right).
- **Infinite Canvas**: The editor is completely unbounded. Columns wrap automatically based on your terminal width.
- **Diatonic Note Mode**: Toggle note mode (`n`) to instantly translate all fret numbers into their corresponding diatonic note letters. 
- **Key Signature Support**: Add textual annotations above columns (e.g., `Key: Bb Minor`). TermTab will intelligently adjust the note translation to use the appropriate sharps or flats for that specific key context moving forward!
- **Undo/Redo**: Complete snapshot-based state tracking.
- **Full File Persistence**: Projects save their entire state—including your cursor position and the complete undo/redo history—so you can pick up exactly where you left off.

## Quick Start

You must provide a filename to launch TermTab. If the file doesn't exist, it will boot up with a fresh 4-measure canvas.

```bash
cargo run my_song.json
```

## Command Cheatsheet

### Navigation (Normal Mode)
- `h, j, k, l`: Move cursor left, down, up, right.
- `w`: Jump forward to the start of the next measure.
- `e`: Jump forward to the end of the current measure.
- `b`: Jump backward to the start of the current/previous measure.
- `[number][command]`: Prefix a command with a number to multiply it (e.g. `10j` or `4w`).

### Editing (Normal Mode)
- `r`: Enter Replace Mode.
- `v`: Enter Visual Mode to highlight columns.
- `y`: Yank (Copy) the selected columns.
- `d` / `x`: Delete (Cut) the selected columns.
- `p`: Paste the clipboard at the cursor.
- `>` / `<`: Insert or delete a blank column at the cursor across all 6 strings.
- `A` (Shift+A): Open a prompt to type an annotation (e.g., chords, lyrics, or Key signatures).
- `n`: Toggle Note Mode.
- `u`: Undo the last action.
- `Ctrl+R`: Redo the last undone action.
- `?`: Open the Help popup.

### Replace Mode
After pressing `r`, you enter Replace Mode where typed characters are placed under the cursor.
- **Numbers**: Standard fret entry. TermTab dynamically consumes horizontal columns if you type a double-digit fret (e.g. `11`).
- **Notation**: Only valid guitar notation characters are allowed: `h`, `p`, `s`, `x`, `b`, `r`, `~`, `t`, `/`, `\`, `-`.
- **Barlines**: Type `|` to insert a full vertical barline. (Fails if the column isn't entirely blank).
- **Exiting**: Press `Esc` or a directional key (`h, j, k, l`) to commit the replacement and return to Normal mode.

### Command Mode
Press `:` in Normal Mode to open the command prompt.
- `:w` - Save the file.
- `:q` - Quit (warns if there are unsaved changes).
- `:q!` - Force quit and discard changes.
- `:wq` - Save and quit.
- `:<number>` - (e.g. `:120`) Instantly jump your cursor to that exact column index.

## Architecture

TermTab does not use a 2D chunked array. Instead, the document is an infinite `Vec<TabColumn>`. The TUI dynamically word-wraps this continuous stream into visual blocks. Because annotations are intrinsically bound to their respective `TabColumn`, shifting columns natively shifts the annotations perfectly without any complex index management. Overlapping annotations are automatically stacked onto new vertical lines by the renderer.

If you manually type two adjacent barlines (`||`), the word-wrapper will dynamically detect it and break the current visual block, giving you absolute control over measure layout!
