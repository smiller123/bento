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

