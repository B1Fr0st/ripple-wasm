# ripple

x86_64 assembly simulator compiled to WebAssembly. Interprets a subset of NASM Intel syntax directly in the browser — no native binary, no server. 

## Goals

- Run x86_64 assembly in a browser with live stdout/stderr output
- Small footprint (~35 KB gzipped WASM)
- Good enough coverage for learning and small programs

## Status

Core instruction set implemented: arithmetic (`add`, `sub`, `imul`, `div`, `neg`), bitwise (`and`, `or`, `xor`, `not`), shifts (`shl`, `shr`, `sar`), data movement (`mov`, `movzx`, `movsx`, `lea`, `push`, `pop`), control flow (`jmp`, all conditional jumps, `call`, `ret`, `loop`). NASM directives: `.data`/`.text` sections, `db`/`dw`/`dd`/`dq`, `times`, `equ`, `global`. Linux syscalls: `read`, `write`, `exit`.

Sub-register access (`al`, `ax`, `eax`, etc.) and all flag semantics (ZF, CF, SF, OF, PF) are handled correctly.

## Usage

Download `asm_sim.js` and `asm_sim_bg.wasm` from the [latest release](../../releases/tag/latest).

```js
import init, { Simulator } from './asm_sim.js';
await init();

const sim = new Simulator(`
section .text
global _start
_start:
    mov rax, 60
    xor rdi, rdi
    syscall
`);

if (sim.error()) throw new Error(sim.error());

sim.run();
console.log(sim.take_stdout());   // buffered stdout
console.log(sim.exit_code());     // 0
```

### API

| Method | Description |
|---|---|
| `new Simulator(source)` | Assemble source; check `error()` before use |
| `error()` | Assembly error string, or `null` |
| `run()` | Run to completion |
| `step()` | Execute one instruction; returns `true` when halted |
| `is_halted()` | Whether execution has ended |
| `exit_code()` | Exit code from `sys_exit` |
| `take_stdout()` | Drain accumulated stdout |
| `take_stderr()` | Drain accumulated stderr |
| `feed_stdin(bytes)` | Supply bytes for `sys_read` |
| `dump_regs()` | Human-readable register dump |
| `steps()` | Total instructions executed |
