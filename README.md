# ripple

x86_64 assembly simulator compiled to WebAssembly. Interprets a subset of NASM Intel syntax directly in the browser — no native binary, no server. 

## Goals

- Run x86_64 assembly in a browser with live stdout/stderr output
- Small footprint (~35 KB gzipped WASM)
- Good enough coverage for learning and small programs

## Supported Operations

### Registers

| Width | Names |
|-------|-------|
| 64-bit | `rax rbx rcx rdx rsi rdi rsp rbp r8`–`r15` |
| 32-bit | `eax ebx ecx edx esi edi esp ebp r8d`–`r15d` |
| 16-bit | `ax bx cx dx si di sp bp r8w`–`r15w` |
| 8-bit low | `al bl cl dl sil dil spl bpl r8b`–`r15b` |
| 8-bit high | `ah bh ch dh` |

All flag semantics (ZF, CF, SF, OF, PF) are handled correctly.

### Instructions

**Data movement**
```nasm
mov   dst, src
movzx dst, src       ; zero-extend
movsx dst, src       ; sign-extend
xchg  dst, src
push  src
pop   dst
lea   dst, [mem]
```

**Arithmetic**
```nasm
add  dst, src
adc  dst, src        ; add with carry
sub  dst, src
sbb  dst, src        ; sub with borrow
inc  dst
dec  dst
neg  dst
mul  src             ; rdx:rax = rax * src (unsigned)
imul src             ; rdx:rax = rax * src (signed)
imul dst, src        ; dst *= src
imul dst, src, imm   ; dst = src * imm
div  src             ; rax=quot, rdx=rem (unsigned)
idiv src             ; rax=quot, rdx=rem (signed)
```

**Bitwise / shift**
```nasm
and  dst, src
or   dst, src
xor  dst, src
not  dst
shl  dst, src   ; alias: sal
shr  dst, src
sar  dst, src
rol  dst, src
ror  dst, src
```

**Comparison**
```nasm
cmp  dst, src
test dst, src
```

**Jumps**
```nasm
jmp  label
je  / jz     jne / jnz
jl  / jnge   jle / jng
jg  / jnle   jge / jnl
js           jns
jc  / jb     jnc / jae
jo           jno
```

**Control flow**
```nasm
call label
ret          ; optional immediate pops extra bytes from stack
loop label   ; dec rcx, jump if rcx != 0
```

**Misc**
```nasm
nop
syscall      ; Linux x86-64 ABI (write/exit at minimum)
enter imm, 0
leave
```

### Memory operand syntax
```nasm
[reg]
[reg + reg]
[reg + reg*2]   ; scale: 1, 2, 4, 8
[reg + disp]
[reg + reg*4 + disp]
[label]
byte ptr [...]  ; word/dword/qword ptr also valid
```

### Directives
```nasm
section .text / .data
global name
db 1, 2, 3, 10    ; bytes
dw / dd / dq      ; 16/32/64-bit values
times N db val
label equ value
.local_label:     ; dot-prefixed local labels
```

## Usage

Download `ripple_wasm.js` and `ripple_wasm_bg.wasm` from the [latest release](../../releases/tag/latest).

```js
import init, { Simulator } from './ripple_wasm.js';
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
| `regs()` | Javascript object containing register state|
| `steps()` | Total instructions executed |
