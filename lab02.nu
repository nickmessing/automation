#!/usr/bin/env nu

const script_dir = path self | path dirname

# where data/ and error.log go, next to the script unless LAB02_ROOT is set
def root []: nothing -> string {
    $env.LAB02_ROOT? | default $script_dir
}
const api = "https://api.frankfurter.dev/v2"

def log-error [context: string, message: string] {
    let line = $"(date now | format date '%Y-%m-%dT%H:%M:%S%:z') [ERROR] ($context): ($message)"
    $line + "\n" | save --append (root | path join "error.log")
}

# log the error to error.log, then raise it
def fail [context: string, msg: string, help?: string] {
    log-error $context $msg
    if $help == null {
        error make --unspanned {msg: $msg}
    } else {
        error make --unspanned {msg: $msg, help: $help}
    }
}

def check-currency [code: string, context: string]: nothing -> string {
    let upper = $code | str upcase
    if not ($upper =~ '^[A-Z]{3}$') {
        fail $context $"'($code)' is not a currency code" "use a 3-letter ISO 4217 code, e.g. EUR, USD, MDL"
    }
    $upper
}

def check-date [value: string, context: string]: nothing -> datetime {
    let parsed = try { $value | into datetime --format "%Y-%m-%d" } catch { null }
    if $parsed == null {
        fail $context $"'($value)' is not a valid date" "use the YYYY-MM-DD format, e.g. 2024-03-15"
    }
    if $parsed > (date now) {
        fail $context $"($value) is in the future" "pick today or an earlier date"
    }
    $parsed
}

def fetch [url: string, context: string] {
    let response = try {
        http get --full --allow-errors $url
    } catch {|err|
        fail $context $"request to ($url) failed: ($err.msg)" "check your internet connection"
    }

    if $response.status != 200 {
        let message = $response.body | get --optional message | default ($response.body | to text)
        let help = match $response.status {
            404 => "no data for this date, it may be before the currency was tracked"
            422 => "check the currency codes and date"
            _ => null
        }
        fail $context $"API error: ($message) \(HTTP ($response.status)\)" $help
    }

    $response.body
}

def save-data [name: string, data: any]: nothing -> string {
    let data_dir = root | path join "data"
    mkdir $data_dir
    let file = $data_dir | path join $"($name).json"
    $data | save --force $file
    $file
}

# braille dot bits for one cell column, rows top to bottom
const left_dots = [0x01 0x02 0x04 0x40]
const right_dots = [0x08 0x10 0x20 0x80]

def dot-bits [span: record, row: int, weights: list<int>]: nothing -> int {
    if $span.bottom < $row or $span.top > ($row + 3) { return 0 }
    0..3 | each {|k|
        let d = $row + $k
        if $d >= $span.top and $d <= $span.bottom { $weights | get $k } else { 0 }
    } | math sum
}

# braille line chart sized to the terminal
def draw-graph [points: table] {
    let size = term size
    let values = $points | get rate
    let n = $values | length
    let min = $values | math min
    let max = $values | math max
    # a flat line would divide by zero, give it some room
    let pad = if $max == $min { ($max | math abs) * 0.01 + 0.0001 } else { 0 }
    let lo = $min - $pad
    let hi = $max + $pad
    let range = $hi - $lo
    # enough decimals to tell rows apart, but never more than the data has
    let data_decimals = $values | each { into string | split row "." | get --optional 1 | default "" | str length } | math max
    let wanted = ($range | math log 10) * -1 | math ceil | $in + 2
    let decimals = [2, ([$wanted, $data_decimals] | math min)] | math max | into int
    let label_width = [$hi $lo] | each { into string --decimals $decimals | str length } | math max

    # header, x axis, dates, stats, saved line and the prompt take ~8 rows
    let rows = [5, ($size.rows - 8)] | math max
    let cols = [10, ($size.columns - $label_width - 2)] | math max
    let dot_cols = $cols * 2
    let dot_rows = $rows * 4

    # sample the series once per dot column, as a dot row (0 = top)
    let ys = 0..<$dot_cols | each {|x|
        let t = if $n == 1 { 0 } else { $x * ($n - 1) / ($dot_cols - 1) }
        let i = $t | math floor | into int
        let j = [($i + 1), ($n - 1)] | math min
        let f = $t - $i
        let v = ($values | get $i) * (1 - $f) + ($values | get $j) * $f
        ($hi - $v) / $range * ($dot_rows - 1) | math round | into int
    }
    # join each sample to the previous one so the line has no gaps
    let spans = $ys | enumerate | each {|it|
        let prev = if $it.index == 0 { $it.item } else { $ys | get ($it.index - 1) }
        {top: ([$prev $it.item] | math min), bottom: ([$prev $it.item] | math max)}
    }

    # a tick every 4 rows from the bottom, plus the top row if there is room
    let ticks = 0..<$rows | each {|r|
        let from_bottom = ($rows - 1 - $r) mod 4
        if $from_bottom == 0 or ($r == 0 and $from_bottom >= 2) {
            let v = $hi - ($r * 4 + 1.5) / ($dot_rows - 1) * $range
            $v | into string --decimals $decimals
        } else {
            ""
        }
    }
    # drop ticks that round to the same label as the one above
    let ticks = $ticks | reduce --fold {shown: "", out: []} {|t, acc|
        if $t == "" or $t == $acc.shown {
            $acc | update out { append "" }
        } else {
            {shown: $t, out: ($acc.out | append $t)}
        }
    } | get out

    $ticks | enumerate | each {|it|
        let row = $it.index * 4
        let line = 0..<$cols | each {|c|
            let mask = (dot-bits ($spans | get ($c * 2)) $row $left_dots) + (dot-bits ($spans | get ($c * 2 + 1)) $row $right_dots)
            if $mask == 0 { " " } else { char --integer (0x2800 + $mask) }
        } | str join

        let axis = if $it.item != "" {
            ($it.item | fill --alignment right --width $label_width) + " ┤"
        } else {
            ("" | fill --width $label_width) + " │"
        }
        print $"(ansi dark_gray)($axis)(ansi reset)(ansi cyan)($line)(ansi reset)"
    } | ignore

    # x axis with a date under each tick
    let k = if $n == 1 { 1 } else { [$n, ([2, (($cols - 1) // 20 + 1)] | math max)] | math min }
    let ticks = 0..<$k | each {|i| if $k == 1 { 0 } else { $i * ($cols - 1) / ($k - 1) | math round | into int } }
    let axis = 0..<$cols | each {|c| if $c in $ticks { "┬" } else { "─" } } | str join
    let labels = $ticks | enumerate | reduce --fold "" {|it, acc|
        let idx = if $cols == 1 { 0 } else { $it.item * ($n - 1) / ($cols - 1) | math round | into int }
        let label = $points | get $idx | get date
        let start = if $it.index == $k - 1 and $k > 1 { $cols - ($label | str length) } else { $it.item }
        if ($acc | str length) > $start { $acc } else { ($acc | fill --width $start) + $label }
    }
    let gutter = "" | fill --width ($label_width + 1)
    print $"(ansi dark_gray)($gutter)└($axis)(ansi reset)"
    print $"($gutter) ($labels)"
}

export def rate [
    base: string,       # currency to convert from, e.g. EUR
    quote: string,      # currency to convert to, e.g. USD
    date?: string,      # date in YYYY-MM-DD format, latest if omitted
    --from: string,     # start of a date range, draws a graph
    --to: string,       # end of the date range, today if omitted
    --no-graph,         # skip the graph (it is also skipped when stdout is not a terminal)
] {
    if $from == null and $to == null {
        single-rate $base $quote $date
    } else {
        range-rate $base $quote $date $from $to (not $no_graph and (is-terminal --stdout))
    }
}

def single-rate [base: string, quote: string, date?: string] {
    let context = $"($base)->($quote) ($date | default 'latest')"
    let base = check-currency $base $context
    let quote = check-currency $quote $context
    if $date != null { check-date $date $context | ignore }

    let url = if $date == null {
        $"($api)/rate/($base)/($quote)"
    } else {
        $"($api)/rate/($base)/($quote)?date=($date)"
    }

    let data = fetch $url $context
    let file = save-data $"($data.base)-($data.quote)-($data.date)" $data

    print $"1 ($data.base) = ($data.rate) ($data.quote) on ($data.date)"
    print $"saved to ($file)"
}

def range-rate [base: string, quote: string, date?: string, from?: string, to?: string, graph: bool = true] {
    let to = $to | default (date now | format date "%Y-%m-%d")
    let context = $"($base)->($quote) ($from | default '?')..($to)"
    if $date != null {
        fail $context "pass either a date or --from/--to, not both"
    }
    if $from == null {
        fail $context "--to needs a --from" "add --from YYYY-MM-DD"
    }
    let base = check-currency $base $context
    let quote = check-currency $quote $context
    if (check-date $from $context) > (check-date $to $context) {
        fail $context $"--from ($from) is after --to ($to)" "swap the dates"
    }

    let data = fetch $"($api)/rates?from=($from)&to=($to)&base=($base)&quotes=($quote)" $context
    if ($data | is-empty) {
        fail $context "API returned no rates for this range" "the range may be before the currency was tracked"
    }

    let first = $data | first
    let last = $data | last
    let file = save-data $"($base)-($quote)-($first.date)_($last.date)" $data

    print $"(ansi white_bold)($base) → ($quote)(ansi reset), ($first.date) … ($last.date), ($data | length) (if ($data | length) == 1 { 'day' } else { 'days' })"
    if $graph { draw-graph $data }

    let low = $data | sort-by rate | first
    let high = $data | sort-by rate | last
    let change = $last.rate - $first.rate
    let pct = $change / $first.rate * 100 | math round --precision 2
    let color = if $change >= 0 { "green" } else { "red" }
    let sign = if $change >= 0 { "+" } else { "" }
    print $"low ($low.rate) \(($low.date)\)  high ($high.rate) \(($high.date)\)  change (ansi $color)($sign)($pct)%(ansi reset)"
    print $"saved to ($file)"
}
