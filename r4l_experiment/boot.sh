#!/bin/sh
kernel_image="../linux/arch/x86/boot/bzImage"

qemu-system-x86_64 \
-netdev user,id=host_net,hostfwd=tcp::7023-:23 \
-device e1000,mac=52:54:00:12:34:50,netdev=host_net \
-kernel $kernel_image \
-append "console=ttyS0" \
-initrd ./initramfs.cpio.gz \
-nographic
