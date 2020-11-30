#!/bin/sh

REPO_DIR=/home/bento/
N_FILES=800
N_FOLDERS=100
PROB=0.05
MAX_DEPTH=2
REPEAT=2

# run benchmark - cp
for mode in cp rsync rsync-checksum bento
do

   for i in {1..5}; do
      # create file image
      cd $REPO_DIR
      cd xv6fs/mkfs
      ./mkfs fs.img
      
      # mount image
      sudo insmod ${REPO_DIR}bentofs/bentofs.ko
      sudo insmod ${REPO_DIR}xv6fs_prov/rust/kernel/xv6fs_prov.ko
      sudo mkdir -p /mnt/xv6fs_prov
      sudo mount -t bentoblk -o loop -o rootmode=40000,user_id=0,group_id=0,blksize=4096,name=xv6fs_ll ${REPO_DIR}xv6fs/mkfs/fs.img /mnt/xv6fs_prov/

      # run benchmark.py
      cd ${REPO_DIR}/bento_backup_utils
      python3 benchmark/benchmark.py  --mode $mode --n-files $N_FILES --n-dirs $N_FOLDERS --repeat $REPEAT --max-depth $MAX_DEPTH --createfile-prob $PROB --modfile-prob $PROB --rmfile-prob $PROB --renamefile-prob $PROB --createdir-prob $PROB --rmdir-prob 0.0 --renamedir-prob $PROB > "${mode}.${i}.txt"

      # unmount image
      sudo umount /mnt/xv6fs_prov
      sudo rmmod xv6fs_prov
      sudo rmmod bentofs
   done
done

