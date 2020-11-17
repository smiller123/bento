import randomfiletree
import pathlib
import shutil
import itertools
import os
import random
import time
import subprocess
from typing import Callable

TEST_DIR = "test_dir"
SRC_DIR = "test_dir/src_dir"
DEST_DIR = "test_dir/dest_dir"

def create_test_dir(n_files: int,
                    n_folders: int,
                    max_depth: int,
                    repeat: int, 
                    payload: Callable) -> None:
   """
   Create a test dir.
   """

   randomfiletree.iterative_gaussian_tree(
      SRC_DIR,
      nfiles=n_files,
      nfolders=n_folders,
      maxdepth=max_depth,
      repeat=repeat,
      payload=callback
   )

def remove_test_dir():
   """
   Remove a test dir.
   """
   shutil.rmtree(TEST_DIR)

def callback(target_dir: pathlib.Path) -> pathlib.Path:
   """
   create a file at target_dir and return its path
   """
   path = target_dir / randomfiletree.core.random_string()

   while True:
      with path.open('w') as f:
         f.write('aaaa')
      yield path

def iterate_files(action: Callable, prob: float):
   """
   iterate over the files in SRC_DIR and perform action
   """
   if not (0 <= prob <= 1):
      raise ValueError("prob must be >= 0 and <=1")

   # TODO: change to pathlib
   for directory, subdirs, files in os.walk(SRC_DIR):
      for filename in files:
         r = random.random()
         if r < prob:
            action(pathlib.Path(directory), filename)

def remove_file(directory: pathlib.Path, filename: str):
   """
   remove filename in directory
   """
   path = directory / filename
   os.remove(path)

def modify_file(directory: pathlib.Path, filename: str):
   """
   modify filename in directory
   """
   path = directory / filename
   with open(path, 'a') as f:
      f.write('b')

def rename_file(directory: pathlib.Path, filename: str):
   """
   rename filename in directory to a new random filename
   """
   src = directory / filename
   dest = directory / randomfiletree.core.random_string()
   os.rename(src, dest)

def iterate_directories(action, prob):
   """
   iterate over subdir in SRC_DIR and perform action
   """
   if not (0 <= prob <= 1):
      raise ValueError("prob must be >= 0 and <=1")

   for directory, subdirs, files in os.walk(SRC_DIR):
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

def run_rsync():
   """
   run rsync on the SRC_DIR
   """
   start_time = time.time()
   subprocess.call(['rsync', '-r', SRC_DIR + "/", DEST_DIR])
   end_time = time.time()
   duration = end_time - start_time
   print('rsync: {} s'.format(duration))

def main():
   if os.path.isdir(TEST_DIR):
      remove_test_dir()

   create_test_dir(
      n_files=5,
      n_folders=5,
      max_depth = 5,
      repeat=3,
      payload=callback)

   iterate_files(modify_file, 0.5)
   iterate_files(remove_file, 0.3)
   iterate_files(rename_file, 0.2)

   iterate_directories(remove_dir, 0.1)
   iterate_directories(rename_dir, 0.1)

   run_rsync()
   remove_test_dir()

if __name__ == "__main__":
   main()