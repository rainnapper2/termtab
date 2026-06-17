Output demos/continuous_replace.gif

Set FontSize 18
Set Width 1200
Set Height 600
Set Padding 20
Set Theme "Catppuccin Macchiato"

# Start termtab
Type "cargo run -- demos/continuous.json"
Enter
Sleep 1s

# Move into position
Type "jjll"
Sleep 500ms

# Enter Continuous Replace mode
Type "R"
Sleep 500ms

# Type a riff with spaces
Type "10 9 8 7"
Sleep 1s

# Add a barline
Type "|"
Sleep 500ms

# Type some more
Type "5 3 0"
Sleep 1s

# Show backspacing over the barline
Backspace 7
Sleep 1s

# Save and Quit
Escape
Type ":wq"
Enter
