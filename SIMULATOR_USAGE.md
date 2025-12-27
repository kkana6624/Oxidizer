# CLI Simulator Usage

The `mdfs` CLI includes a simulation mode that allows you to verify the behavior of your chart textually.

## Command

```bash
cargo run -p mdfs_cli -- simulate <input_file>
```

- `<input_file>`: Path to a `.mdfs` source file or a compiled `.mdf.json` file.
    - If a `.mdfs` file is provided, it will be compiled in-memory first.

## Output Format

The simulator outputs a time-ordered list of events.

```text
Simulation Start (4200000 us total)
Time(us) | S 1 2 3 4 5 6 7 | Info
---------|-----------------|------------------
       0 | N . . . . . . . | BPM: 150.0
  100000 | . . N . . . . . |
  400000 | . C . . . . . N |
  500000 | . | . . . . . N |
  700000 | . # . . . . . N |
```

### Columns

1.  **Time(us)**: Absolute time in microseconds.
2.  **Lanes (S 1-7)**: Visual representation of notes on each lane.
    -   `S`: Scratch Lane (Col 0)
    -   `1-7`: Key Lanes (Col 1-7)
    -   **Symbols**:
        -   `.`: Empty
        -   `N`: Tap Note
        -   `C`: Charge Note Start
        -   `H`: Hell Charge Note Start
        -   `B`: Back Spin Scratch Start
        -   `b`: Hell BSS Start
        -   `M`: Multi Spin Scratch Start
        -   `m`: Hell MSS Start
        -   `|`: Holding (continuation)
        -   `#`: Hold End
        -   `!`: MSS Checkpoint (Reverse required)
3.  **Info**: Additional events happening at this time.
    -   `BPM: <value>`: BPM change.
    -   `BGM x<count>`: Number of background sound events triggered.

## Example

Given `example.mdfs`:

```text
track: |
    @bpm 150
    @div 16
    S.......
    ........
    m....... @rev_every 4
    ........
    ........
    ........
    ........
    m.......
```

Running simulation:

```bash
cargo run -p mdfs_cli -- simulate example.mdfs
```

Output:

```text
Simulation Start (500000 us total)
Time(us) | S 1 2 3 4 5 6 7 | Info
---------|-----------------|------------------
       0 | N . . . . . . . | BPM: 150.0
  200000 | M . . . . . . . |
  600000 | ! . . . . . . . |
 1000000 | # . . . . . . . |
```
