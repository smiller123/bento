# Bento Backup Utility

This project is developed by Teerapat Jenrungrot, Pat Kosakanchit, Nicholas Monsees.

## Getting started

1. Mount `xv6fs_prov` (i.e., to `/mnt/xv6fs_prov`).
2. Run `cargo build`.
3. Create the source directory (e.g., `/mnt/xv6fs_prov/src_dir`).
4. Make any changes in the source directory.
5. Run `cargo run /mnt/xv6fs_prov/ /mnt/xv6fs_prov/src_dir ./dest_dir/`.
6. (Optional) Repeat Steps 4 and 5.

## Benchmark

To run the benchmark, run the following code.

```
cd benchmark
pip install -r requirements.txt
python benchmark.py --mode bento
```

To generate the data in .csv format, use --csv option. The program will print comma-separated values which can be imported directly into the Google sheet.

If you want to automate the process of running benchmark.py, run run_benchmark.sh using sudo. The script run benchmark.py on all the modes. Change the repo dir and other settings before running the script. The output will be appended to {N_FILES}-{N_FOLDERS}.txt file. Note that the script assumes that bento is already cloned and compiled.