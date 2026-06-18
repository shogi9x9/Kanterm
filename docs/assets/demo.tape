# Regenerate the demo GIF with:
#   scripts/seed-demo.sh        # builds /tmp/kanban-demo.db
#   vhs docs/assets/demo.tape   # writes docs/assets/demo.gif

Output docs/assets/demo.gif

Set Shell "bash"
Set FontSize 16
Set Width 1300
Set Height 720
Set Padding 20
Set Theme "Dracula"

Hide
Type "KANBAN_DB=/tmp/kanban-demo.db KANBAN_THEME=dark ./target/release/kanban-tui"
Enter
Sleep 800ms
Show

Sleep 1.4s
Down           # move selection within the column
Sleep 600ms
Up
Sleep 500ms

Enter          # open the card detail modal
Sleep 1.6s
Type "m"       # agent execution metadata (weight / effort / model / tokens / gate)
Sleep 2.6s
Escape         # back to detail
Sleep 600ms
Escape         # back to the board
Sleep 700ms

Type "L"       # push the selected card one column to the right
Sleep 1.2s
Type "H"       # and pull it back
Sleep 900ms

Type "g"       # dependency graph: edges, executable stages, blockers
Sleep 3s
Escape
Sleep 700ms

Type "/"       # filter cards by title / body / label
Sleep 500ms
Type "graph"
Sleep 1.3s
Escape         # clear filter
Sleep 700ms

Type "q"       # quit
Sleep 500ms
