Output docs/img/demo.gif
Output docs/es/img/demo.gif

Set Shell "bash"
Set FontSize 16
Set Width 1500
Set Height 760
Set Padding 20
Set Theme "Dracula"
Set TypingSpeed 60ms

Hide
Type "export PATH=$PWD/.venv/bin:$PATH PS1='$ ' && cd tests/fixtures/demo && rm -rf .complexipy_cache && clear"
Enter
Show

Type "complexipy . --max-complexity-allowed 10"
Sleep 500ms
Enter
Sleep 3s

Type "complexipy . --max-complexity-allowed 10 --failed --suggest-refactors"
Sleep 500ms
Enter
Sleep 6s
