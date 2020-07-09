#include <linux/init.h>
#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/slab.h>
#include <linux/bug.h>

char __morestack[1024];
char _GLOBAL_OFFSET_TABLE_;

void abort(void)
{
    BUG();
}

extern void rust_main(void);
extern void rust_exit(void);

static int xv6fs_init(void)
{
    rust_main();
    return 0;
}

static void xv6fs_exit(void)
{
    rust_exit();
}

module_init(xv6fs_init);
module_exit(xv6fs_exit);

MODULE_LICENSE("MIT");
