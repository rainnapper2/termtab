#!/bin/bash
set -e

cargo build

echo "Generating Tapes..."

run_demo() {
    local tape_file=$1
    local json_file="${tape_file%.tape}.json"
    
    if [ "$tape_file" = "demos/2_editing.tape" ] || [ "$tape_file" = "demos/3_undo_redo.tape" ] || [ "$tape_file" = "demos/9_chords.tape" ]; then
        rm -f "$json_file"
    else
        cp demos/song.json "$json_file"
    fi
    vhs "$tape_file"
}

if [ -n "$1" ]; then
    run_demo "$1"
else
    run_demo demos/1_navigation.tape
    run_demo demos/2_editing.tape
    run_demo demos/3_undo_redo.tape
    run_demo demos/4_copy_paste.tape
    run_demo demos/5_note_mode.tape
    run_demo demos/6_annotations.tape
    run_demo demos/7_key_change.tape
    run_demo demos/8_dynamic_wrap.tape
    run_demo demos/readme_demo.tape
    run_demo demos/9_chords.tape
fi

echo "All Done!"
