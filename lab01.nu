#!/usr/bin/env nu

def ensure-empty-folder [path: string]: nothing -> string {
    let expanded_path = $path | path expand
    match ($expanded_path | path type) {
        null => { mkdir --verbose $expanded_path }
        "dir" => {
            if not (ls --all $expanded_path | is-empty) {
                error make --unspanned {
                    msg: $"'($expanded_path)' is not empty"
                    help: "remove its contents or pick another path"
                }
            }
        }
        $t => {
            error make --unspanned {msg: $"'($expanded_path)' is not a folder \(it is a ($t)\)"}
        }
    }

    $expanded_path
}

def ensure-source-folder [path: string]: nothing -> string {
    let expanded_path = $path | path expand
    match ($expanded_path | path type) {
        null => {
            error make --unspanned {
                msg: $"source '($expanded_path)' does not exist"
                help: "check the path, or pass a different source"
            }
        }
        "dir" => {
            if (ls --all $expanded_path | is-empty) {
                error make --unspanned {msg: $"source '($expanded_path)' is empty, nothing to back up"}
            }
        }
        $t => {
            error make --unspanned {msg: $"source '($expanded_path)' is not a folder \(it is a ($t)\)"}
        }
    }

    $expanded_path
}
def walk [dir: string]: nothing -> table {
    let entries = ls --all --full-paths $dir
    let here = $entries | where type != dir
    let deeper = $entries | where type == dir | get name | each {|d| walk $d } | flatten

    $here | append $deeper
}

def walk-dirs [dir: string]: nothing -> list<string> {
    ls --all --full-paths $dir
    | where type == dir
    | get name
    | each {|d| [$d] | append (walk-dirs $d) }
    | flatten
}

def draw-progress [done: int, total: int, label: string, width: int = 28] {
    let ratio = if $total == 0 { 1.0 } else { $done / $total }
    let filled = ($ratio * $width) | math round | into int
    let bar = (
        (ansi green_bold) + ("" | fill --character "━" --width $filled)
        + (ansi cyan) + ("" | fill --character "─" --width ($width - $filled))
        + (ansi reset)
    )
    let pct = ($ratio * 100) | math round | into int

    let shown = if ($label | str length) > 40 {
        "…" + ($label | str substring (($label | str length) - 39)..)
    } else {
        $label
    }

    print --no-newline $"(ansi --escape '2K')\r($bar) ($pct)% ($done)/($total) ($shown)"
}

export def backup [
  source: string,                   # source directory to backup
  destination: string = "/backup",  # destination directory to backup to
] {
    let source_full_path = ensure-source-folder $source
    let destination_full_path = $destination | path expand

    if $destination_full_path == $source_full_path or ($destination_full_path | str starts-with ($source_full_path + (char path_sep))) {
        error make --unspanned {
            msg: "destination is inside the source"
            help: "pick a destination outside the folder you are backing up"
        }
    }

    ensure-empty-folder $destination_full_path | ignore

    print $"backup ($source_full_path) to ($destination_full_path)"

    let files = walk $source_full_path
    let total = $files | length
    let bytes = $files | get size | math sum
    let interactive = is-terminal --stdout

    print $"($total) files, ($bytes)"

    walk-dirs $source_full_path
    | each {|d| mkdir ($destination_full_path | path join ($d | path relative-to $source_full_path)) }

    $files | enumerate | each {|it|
        let rel = $it.item.name | path relative-to $source_full_path
        let target = $destination_full_path | path join $rel

        if $interactive {
            draw-progress $it.index $total $rel
        }

        # `--no-dereference` for symlinks
        cp --no-dereference $it.item.name $target
    }

    if $interactive { draw-progress $total $total "done" }
    print ""
    print $"copied ($total) files \(($bytes)\) to ($destination_full_path)"
}
