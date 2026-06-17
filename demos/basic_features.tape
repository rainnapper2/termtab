Output demos/basic_features.gif

Set FontSize 18
Set Width 1200
Set Height 600
Set Padding 20
Set Theme "Catppuccin Macchiato"

# Start termtab
Type "cargo run -- demos/basic.json"
Enter
Sleep 1s

# Show navigation
Type "jjll"
Sleep 500ms
Type "kk"
Sleep 500ms
Type "e"
Sleep 500ms
Type "w"
Sleep 500ms
Type "b"
Sleep 500ms

# Insert a note
Type "r7"
Sleep 500ms

# Copy and Paste using Visual Mode
Type "v"
Sleep 500ms
Type "lllll"
Sleep 500ms
Type "y"
Sleep 500ms
Type "w"
Sleep 500ms
Type "p"
Sleep 1s

# Save and Quit
Escape
Type ":wq"
Enter
