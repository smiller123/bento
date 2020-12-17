"""
Bento Backup Utility Benchmark

Copyright 2020 Teerapat Jenrungrot, Pat Kosakanchit, Nicholas Monsees

Permission is hereby granted, free of charge, to any person obtaining a copy of this software
and associated documentation files (the "Software"), to deal in the Software without restriction,
including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense,
and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so,
subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial
portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT
LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
"""
import argparse
import pathlib
import shutil
import os
import random
import time
import subprocess
from typing import Callable
import randomfiletree
import checksumdir

BASE_DIR = "/mnt/xv6fs_prov"
SRC_DIR = "/mnt/xv6fs_prov/src_dir"
DEST_DIR = "dest_dir"


def create_test_dir(src_dir: str, n_files: int, n_folders: int, max_depth: int,
                    repeat: int, payload: Callable) -> None:
    """
    Create a test dir.
    """

    all_dirs, all_files = randomfiletree.iterative_tree(
        src_dir,
        nfolders_func=lambda _: n_folders,
        nfiles_func=lambda _: n_files,
        maxdepth=max_depth,
        repeat=repeat,
        payload=payload)

    return all_dirs, all_files


def remove_test_dir(path: str) -> None:
    """
    Remove a test dir.
    """
    shutil.rmtree(path)


def callback(target_dir: pathlib.Path) -> pathlib.Path:
    """
    create a file at target_dir and return its path
    """
    while True:
        path = target_dir / randomfiletree.core.random_string()
        with path.open('w') as f:
            f.write('aaaa')
        yield path


def iterate_files(action: Callable, src_dir: str, prob: float) -> None:
    """
    iterate over the files in SRC_DIR and perform action
    """
    if not 0 <= prob <= 1:
        raise ValueError("prob must be >= 0 and <=1")

    for directory, _, files in os.walk(src_dir):
        for filename in files:
            r = random.random()
            if r < prob:
                action(pathlib.Path(directory), filename)


def remove_file(directory: pathlib.Path, filename: str) -> None:
    """
    remove filename in directory
    """
    path = directory / filename
    os.remove(path)


def modify_file(directory: pathlib.Path, filename: str) -> None:
    """
    modify filename in directory
    """
    path = directory / filename
    with open(path, 'a') as f:
        f.write('b')


def rename_file(directory: pathlib.Path, filename: str) -> None:
    """
    rename filename in directory to a new random filename
    """
    src = directory / filename
    dest = directory / randomfiletree.core.random_string()
    os.rename(src, dest)


def iterate_directories(action: Callable, src_dir: str, prob: float) -> None:
    """
    iterate over subdir in SRC_DIR and perform action
    """
    if not 0 <= prob <= 1:
        raise ValueError("prob must be >= 0 and <=1")

    for directory, subdirs, _ in os.walk(src_dir):
        for subdir in subdirs:
            r = random.random()
            if r < prob:
                action(pathlib.Path(directory), subdir)


def create_file(directory: pathlib.Path, subdir: str) -> None:
    """
    create filename in directory
    """
    path = directory / subdir / randomfiletree.core.random_string()
    with open(path, 'w') as f:
        f.write('n')


def create_dir(directory: pathlib.Path, subdir: str) -> None:
    """
    create filename in directory
    """
    path = directory / subdir / randomfiletree.core.random_string()
    os.mkdir(path)


def remove_dir(directory: pathlib.Path, subdir: str):
    """
    remove the specified subdir in directory
    """
    shutil.rmtree(directory / subdir)


def rename_dir(directory: pathlib.Path, subdir: str):
    """
    rename the specified subdir in directory
    """
    src = directory / subdir
    dest = directory / randomfiletree.core.random_string()
    os.rename(src, dest)


def get_tree(src_dir: str) -> dict:
    src_dir_tree = {}
    for directory, subdirs, files in os.walk(src_dir):
        for filename in files:
            file_path = directory + "/" + filename
            with open(file_path) as f:
                file_content = f.read()
            src_dir_tree[file_path] = file_content
        for subdir in subdirs:
            dir_path = directory + "/" + subdir
            src_dir_tree[dir_path] = "<dir>"

    return src_dir_tree


def compare_trees(tree1: dict, tree2: dict) -> (int, int):
    modified_files = 0
    modified_dirs = 0
    for path, content in tree1.items():
        if path not in tree2.keys():
            if content == '<dir>':
                modified_dirs += 1
            else:
                modified_files += 1
        else:
            if content != tree2[path]:
                modified_files += 1

    for path, content in tree2.items():
        if path not in tree1.keys():
            if content == '<dir>':
                modified_dirs += 1
            else:
                modified_files += 1
        else:
            if content != tree1[path]:
                modified_files += 1
    return (modified_files, modified_dirs)


def modify(args: argparse.Namespace) -> None:
    """
    modify contents of the source folders
    """

    tree1 = get_tree(args.src_path)
    # subprocess.call(['tree', args.src_path])

    # Make modifications on src_path
    iterate_files(modify_file, args.src_path, args.modfile_prob)
    iterate_files(remove_file, args.src_path, args.rmfile_prob)
    iterate_files(rename_file, args.src_path, args.renamefile_prob)
    iterate_directories(remove_dir, args.src_path, args.rmdir_prob)
    iterate_directories(rename_dir, args.src_path, args.renamedir_prob)

    iterate_directories(create_dir, args.src_path, args.createdir_prob)
    iterate_directories(create_file, args.src_path, args.createfile_prob)

    tree2 = get_tree(args.src_path)
    # subprocess.call(['tree', args.src_path])

    modified_files, modified_dirs = compare_trees(tree1, tree2)
    if not args.csv:
        print("{} files and {} folders modified".format(modified_files,
                                                    modified_dirs))
    return modified_files, modified_dirs


def run_rsync(args: argparse.Namespace, checksum: bool = False) -> None:
    """
    run rsync on the source directory
    if checksum is true, skip based on checksum instead of modified time and
    filesize
    """
    src_path = args.src_path
    dest_path = args.dest_path

    # backup before
    if checksum:
        subprocess.call(['rsync', '-c', '-r', src_path + "/", dest_path])
    else:
        subprocess.call(['rsync', '-r', src_path + "/", dest_path])

    if not args.csv:
        print("src path hash: {}".format(checksumdir.dirhash(src_path)))

    # modify
    m_files, m_dirs = modify(args)

    # benchmark
    start_time = time.time()
    if checksum:
        subprocess.call([
            'rsync', '--delete', '--checksum', '--recursive', src_path + "/",
            dest_path
        ])
    else:
        subprocess.call(
            ['rsync', '--delete', '--recursive', src_path + "/", dest_path])
    end_time = time.time()
    duration = end_time - start_time

    if not args.csv:
        print('rsync{}: {} s'.format("-checksum" if checksum else "", duration))
        print("src path hash: {}".format(checksumdir.dirhash(src_path)))
        print("dest path hash: {}".format(checksumdir.dirhash(dest_path)))

    return duration, m_files, m_dirs


def run_cp(args: argparse.Namespace) -> None:
    """
    run cp on the source directory
    """
    src_path = args.src_path
    dest_path = args.dest_path

    # backup before
    subprocess.call(['cp', '-rf', src_path + "/", dest_path])
    if not args.csv:
        print("src path hash: {}".format(checksumdir.dirhash(src_path)))

    # modify
    m_files, m_dirs = modify(args)

    # benchmark
    start_time = time.time()
    subprocess.call(['rm', '-rf', dest_path])
    subprocess.call(['cp', '-rf', src_path + "/", dest_path])
    end_time = time.time()
    duration = end_time - start_time
    if not args.csv:
        print('cp: {} s'.format(duration))
        print("src path hash: {}".format(checksumdir.dirhash(src_path)))
        print("dest path hash: {}".format(checksumdir.dirhash(dest_path)))

    return duration, m_files, m_dirs


def run_bento(args: argparse.Namespace) -> None:
    mount_path = args.mount_path
    src_path = args.src_path
    dest_path = args.dest_path
    if not args.csv:
        subprocess.call(['cargo', 'build'])
        subprocess.call(['cargo', 'run', mount_path, src_path, dest_path])
        print("src path hash: {}".format(checksumdir.dirhash(src_path)))
    else:
        subprocess.call(['cargo', 'build'],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.STDOUT)
        subprocess.call(['cargo', 'run', mount_path, src_path, dest_path],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.STDOUT)

    # modify
    m_files, m_dirs = modify(args)

    # start benchmarking
    start_time = time.time()
    if not args.csv:
        subprocess.call(['cargo', 'run', mount_path, src_path, dest_path])
    else:
        subprocess.call(['cargo', 'run', mount_path, src_path, dest_path],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.STDOUT)

    end_time = time.time()
    duration = end_time - start_time
    if not args.csv:
        print("bento: {} s".format(duration))
        print("src path hash: {}".format(checksumdir.dirhash(src_path)))
        print("dest path hash: {}".format(checksumdir.dirhash(dest_path)))

    return duration, m_files, m_dirs


def main(args: argparse.Namespace) -> None:

    random.seed(args.seed)
    # Remove target directory if it exists beforehand
    if os.path.isdir(args.src_path):
        remove_test_dir(args.src_path)

    if os.path.isdir(args.dest_path):
        remove_test_dir(args.dest_path)

    # Create a random directory tree
    all_dirs, all_files = create_test_dir(args.src_path,
                                          n_files=args.n_files,
                                          n_folders=args.n_dirs,
                                          max_depth=args.max_depth,
                                          repeat=args.repeat,
                                          payload=callback)
    if not args.csv:
        print('{} files and {} folders created'.format(len(all_files),
                                                   len(all_dirs)))

    # Run benchmark
    if args.mode == 'rsync':
        duration, m_files, m_dirs = run_rsync(args)
    elif args.mode == 'rsync-checksum':
        duration, m_files, m_dirs = run_rsync(args, checksum=True)
    elif args.mode == 'bento':
        duration, m_files, m_dirs = run_bento(args)
    elif args.mode == 'cp':
        duration, m_files, m_dirs = run_cp(args)
    else:
        raise NotImplementedError("mode not yet supported")

    if not args.skip_cleanup:
        remove_test_dir(args.src_path)
        remove_test_dir(args.dest_path)

    if args.csv:
        print("{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}".format(
            args.mode,
            args.seed,
            args.n_files,
            args.n_dirs,
            args.max_depth,
            args.repeat,
            args.createfile_prob,
            args.modfile_prob,
            args.rmfile_prob,
            args.renamefile_prob,
            args.createdir_prob,
            args.rmdir_prob,
            args.renamedir_prob,
            len(all_files),
            len(all_dirs),
            m_files,
            m_dirs,
            duration))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument('--mount-path',
                        type=str,
                        default=BASE_DIR,
                        help="Mount directory")
    parser.add_argument('--src-path',
                        type=str,
                        default=SRC_DIR,
                        help="Source directory")
    parser.add_argument('--dest-path',
                        type=str,
                        default=DEST_DIR,
                        help="Destination directory")
    parser.add_argument('--mode',
                        type=str,
                        default='rsync',
                        help="'cp' or 'rsync' or 'rsync-checksum' or 'bento'")

    # Directory tree parameters
    parser.add_argument('--n-files',
                        type=int,
                        default=5,
                        help="Number of files to create")
    parser.add_argument('--n-dirs',
                        type=int,
                        default=1,
                        help="Number of folders to create")
    parser.add_argument('--max-depth',
                        type=int,
                        default=1,
                        help="Maximum depth to descend into the file tree")
    parser.add_argument(
        '--repeat',
        type=int,
        default=1,
        help="Number of rounds to repeat file and folders creation")

    # File/directory modification parameters
    parser.add_argument('--seed', type=int, default=3, help="Random seed")
    parser.add_argument('--createfile-prob',
                        type=float,
                        default=0.2,
                        help="The probability of creating a file [0-1]")
    parser.add_argument('--modfile-prob',
                        type=float,
                        default=0.2,
                        help="The probability of modifying a file [0-1]")
    parser.add_argument('--rmfile-prob',
                        type=float,
                        default=0.2,
                        help="The probability of removing a file [0-1]")
    parser.add_argument('--renamefile-prob',
                        type=float,
                        default=0.1,
                        help="The probability of renaming a file [0-1]")
    parser.add_argument('--createdir-prob',
                        type=float,
                        default=0.2,
                        help="The probability of creating a directory [0-1]")
    parser.add_argument('--rmdir-prob',
                        type=float,
                        default=0.2,
                        help="The probability of removing a directory [0-1]")
    parser.add_argument('--renamedir-prob',
                        type=float,
                        default=0.2,
                        help="The probability of renaming a directory [0-1]")
    parser.add_argument(
        '--skip-cleanup',
        action='store_true',
        help="Whether to remove tmp directory tree after benchmark")
    parser.add_argument(
        '--csv',
        action='store_true',
        help="Print the result in csv format"
    )
    args = parser.parse_args()
    main(args)
