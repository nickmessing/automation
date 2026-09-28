# Lab 01 — backup

```
$> use lab01.nu
$> lab01 backup --help
Usage:
  > backup <source> (destination)

Flags:
  -h, --help: Display the help message for this command

Command Type:
  > custom

Parameters:
  source <string>: source directory to backup
  destination <string>: destination directory to backup to (optional, default: '/backup')
```

# Lab 02 — exchange rates

Fetches rates from the [Frankfurter API](https://frankfurter.dev) and saves them to `data/`:
a single date goes to `<BASE>-<QUOTE>-<DATE>.json`, a range to `<BASE>-<QUOTE>-<FROM>_<TO>.json`.
A range also draws a braille line chart that fills the terminal, with the y axis fitted to the data.
Errors (bad currency, bad or future date, empty range, network failure) are printed and appended to `error.log`.

```
$> use lab02.nu
$> lab02 rate EUR USD 2024-03-15
1 EUR = 1.0902 USD on 2024-03-15
saved to .../data/EUR-USD-2024-03-15.json
$> lab02 rate EUR JPY --from 2024-01-01 --to 2024-12-31
EUR → JPY, 2024-01-01 … 2024-12-31, 366 days
174.42 ┤                                    ⢠⠼⡇
       │                                    ⡏ ⢳⡀
       │                                   ⡴⠃  ⣇⡀
170.83 ┤                             ⣤⣀   ⢸⠁   ⠛⡇
       │                           ⢠⠏⠉⠹⢶⣀⣸⠉     ⢳
       │                       ⣀⡀ ⣸⠉    ⠈⠁      ⢸
       │                       ⡇⡇⢸⠁             ⢸⣤
166.05 ┤                      ⢰⠃⢧⡏               ⢸                 ⢰⠦⣄
       │               ⢀⡀ ⢀⣠⡄⣠⠞ ⠘⠃               ⠈⡇               ⢀⡏ ⠘⣆⡀       ⣀
       │          ⢀⣀⢀⡀ ⢸⠉⠳⣼⠉⠙⠃                    ⡇  ⣀            ⡏   ⠉⠉⡇     ⡴⠛
       │          ⡼⠈⠛⡇ ⡼                          ⡇ ⢠⠿⢤       ⢀⡞⠉⠓⠃     ⣇    ⢸⠁
161.26 ┤   ⢰⢲⡀  ⣠⠏⠁  ⠙⠚⠁                          ⢹ ⡼ ⠈⢧⣿   ⢀⣀⡼         ⠈⡇  ⢰⠋
       │  ⣀⡼ ⠉⣇⡴⠃                                 ⢸⡼⠁   ⠸⡄ ⢀⡞⢸⡇          ⢳ ⢠⠏
       │ ⣀⡏⠁  ⠉                                   ⢸⡇     ⢧ ⢸             ⠘⣦⠞
       │ ⡏⠁                                              ⠈⡇⢸              ⠛
156.48 ┤⠒⠃                                                ⠹⠏
       └┬───────────────────────┬──────────────────────┬───────────────────────┬
        2024-01-01              2024-05-03             2024-08-30     2024-12-31
low 156.03 (2024-09-16)  high 174.87 (2024-07-11)  change +4.61%
saved to .../data/EUR-JPY-2024-01-01_2024-12-31.json
$> lab02 rate --help
Usage:
  > rate {flags} <base> <quote> (date)

Flags:
  --from <string>: start of a date range, draws a graph
  --to <string>: end of the date range, today if omitted

Parameters:
  base <string>: currency to convert from, e.g. EUR
  quote <string>: currency to convert to, e.g. USD
  date <string>: date in YYYY-MM-DD format, latest if omitted (optional)
```

# Lab 03 — scheduled exchange rates

Runs lab02 on a schedule in a Fedora container (supercronic, podman or docker), published to
`harbor.nickmessing.com/public/automation-lab03`. See [lab03/readme.md](lab03/readme.md).

```
$> cd lab03
$> podman compose up -d --build     # or: docker compose up -d --build
$> podman compose logs -f
```
