# TODO: Order import by PEP8 standards
import argparse
import randomfiletree
import pathlib
import shutil
import itertools
import os
import random
import time
import subprocess
from typing import Callable

SRC_DIR = "src_dir"
DEST_DIR = "dest_dir"

def create_test_dir(src_dir: str,
                    n_files: int,
                    n_folders: int,
                    max_depth: int,
                    repeat: int,
                    payload: Callable) -> None:
   """
   Create a test dir.
   """
   randomfiletree.iterative_gaussian_tree(
      src_dir,
      nfiles=n_files,
      nfolders=n_folders,
      maxdepth=max_depth,
      repeat=repeat,
      payload=callback,
   )

def remove_test_dir(path: str) -> None:
   """
   Remove a test dir.
   """
   shutil.rmtree(path)

def callback(target_dir: pathlib.Path) -> pathlib.Path:
   """
   create a file at target_dir and return its path
   """
   path = target_dir / randomfiletree.core.random_string()

   while True:
      with path.open('w') as f:
         f.write('aaaa')
      yield path

def iterate_files(action: Callable, src_dir: str, prob: float) -> None:
   """
   iterate over the files in SRC_DIR and perform action
   """
   if not (0 <= prob <= 1):
      raise ValueError("prob must be >= 0 and <=1")

   # TODO: change to pathlib
   for directory, subdirs, files in os.walk(src_dir):
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
   if not (0 <= prob <= 1):
      raise ValueError("prob must be >= 0 and <=1")

   for directory, subdirs, files in os.walk(src_dir):
      for subdir in subdirs:
         r = random.random()
         if r < prob:
            action(pathlib.Path(directory), subdir)

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

def run_rsync(src_path: str, dest_path: str) -> None:
   """
   run rsync on the source directory
   """
   start_time = time.time()
   subprocess.call(['rsync', '-r', src_path + "/", dest_path])
   end_time = time.time()
   duration = end_time - start_time
   print('rsync: {} s'.format(duration))

def run_bento(src_path: str, dest_path: str) -> None:
   start_time = time.time()
   # TODO: get a list of files to be updated / removed

   # TODO: filter the list based on src_path and dest_path

   # TODO: run fs copy

   end_time = time.time()
   duration = end_time - start_time
   print("bento: {} s".format(duration))

def main(args: argparse.Namespace) -> None:
   # Remove target directory if it exists beforehand
   if os.path.isdir(args.src_path):
      remove_test_dir(args.src_path)

   if os.path.isdir(args.dest_path):
      remove_test_dir(args.dest_path)

   # Create a random directory tree
   create_test_dir(args.src_path,
                   n_files=args.n_files,
                   n_folders=args.n_dirs,
                   max_depth=args.max_depth,
                   repeat=args.max_depth,
                   payload=callback)

   # TODO: copy src_path to dest_path

   # Make modifications on src_path
   iterate_files(modify_file, args.src_path, args.modfile_prob)
   iterate_files(remove_file, args.src_path, args.rmfile_prob)
   iterate_files(rename_file, args.src_path, args.renamefile_prob)
   iterate_directories(remove_dir, args.src_path, args.rmdir_prob)
   iterate_directories(rename_dir, args.src_path, args.renamedir_prob)

   # Run benchmark
   if args.mode == 'rsync':
      run_rsync(args.src_path, args.dest_path)
   elif args.mode == 'bento':
      run_bento(args.src_path, args.dest_path)
   else:
      raise NotImplementedError("mode not yet supported")

   if not args.skip_cleanup:
      remove_test_dir(args.src_path)
      remove_test_dir(args.dest_path)

if __name__ == "__main__":
   parser = argparse.ArgumentParser()
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
                       help="'rsync' or 'bento'")

   # Directory tree parameters
   parser.add_argument('--n-files',
                       type=int,
                       default=5,
                       help="")
   parser.add_argument('--n-dirs',
                       type=int,
                       default=5,
                       help="")
   parser.add_argument('--max-depth',
                       type=int,
                       default=5,
                       help="")
   parser.add_argument('--repeat',
                       type=int,
                       default=3,
                       help="")

   # File/directory modification parameters
   parser.add_argument('--modfile-prob',
                       type=float,
                       default=0.5,
                       help="")
   parser.add_argument('--rmfile-prob',
                       type=float,
                       default=0.3,
                       help="")
   parser.add_argument('--renamefile-prob',
                       type=float,
                       default=0.3,
                       help="")
   parser.add_argument('--rmdir-prob',
                       type=float,
                       default=0.3,
                       help="")
   parser.add_argument('--renamedir-prob',
                       type=float,
                       default=0.3,
                       help="")
   parser.add_argument('--skip-cleanup',
                       type=bool,
                       default=False,
                       help="Whether to remove tmp directory tree after benchmark")
   args = parser.parse_args()
   main(args)
