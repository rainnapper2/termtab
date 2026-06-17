Output demos/build_song.gif
Set FontSize 18
Set Width 1200
Set Height 600
Set Theme "Catppuccin Macchiato"

Type "cargo run -- demos/song.json"
Enter
Sleep 1s

# Go to A string (string 4, so down 4 from top e)
Type "jjjj"
# Write some chords and a very long run of notes to force wrapping
Type "R"
Type "0 2 3 | 0 2 3 | 0 2 3 | 0 2 3 | 0 2 3 | 0 2 3 | 0 2 3 | 0 2 3 | 0 2 3 | 0 2 3 | "
Escape
Type "kR2 4 5 | 2 4 5 | 2 4 5 | 2 4 5 | 2 4 5 | 2 4 5 | 2 4 5 | 2 4 5 | 2 4 5 | 2 4 5 | "
Escape
Type "kR2"
Escape
Type "lR"
Type "3"
Escape
Type "jR0"
Escape
Type "jR2"
Escape
# Add an A# (G string, 3rd fret)
Type "kk"
Type "R"
Type "3"
Escape

Type ":wq"
Enter
