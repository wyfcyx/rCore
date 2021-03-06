// See LICENSE for license details.

#include "config.h"
#include "encoding.h"

#if CONFIG_WITH_DTB
  .section ".payload.dtb","a",@progbits

  .align RISCV_PGSHIFT
  .globl __dtb_start, __dtb_end
__dtb_start:
  .incbin BBL_PAYLOAD_DTB
__dtb_end:
#endif

  .section ".payload","a",@progbits

#if RELAXED_ALIGNMENT
  /* align payload minimally */
  .align 3
#else
  /* align payload to megapage */
  .align RISCV_PGSHIFT + RISCV_PGLEVEL_BITS
#endif

  .globl _payload_start, _payload_end
_payload_start:
  .incbin BBL_PAYLOAD
_payload_end:
