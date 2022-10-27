.altmacro
.macro SAVE n
	sd x\n, \n*8(sp)
.endm
// 保存 x1, x3-x31, x2 是 sp 不保存
.macro SAVE_ALL
	sd x1, 1*8(sp)
	.set n, 3
	.rept 29
		SAVE %n
		.set n, n+1
	.endr
.endm

.macro LOAD n
	ld x\n, \n*8(sp)
.endm
.macro LOAD_ALL
	ld x1, 1*8(sp)
	.set n, 3
	.rept 29
		LOAD %n
		.set n, n+1
	.endr
.endm
	.globl yield_naked
	.globl execute_naked

execute_naked:
	// 位置无关加载
	// 保存调度上下文(x1, x3-x31 30 个寄存器), 在调度函数的栈上
	addi sp, sp, -32*8
	// 保存除 sp、x0 以外的 x 系列寄存器
	SAVE_ALL
	// 设置陷入入口 标签为 1 (用于返回到调度函数)
	la   t0, 1f
	csrw stvec, t0
	// 保存调度上下文地址并切换上下文
	// sscratch 的值为 LocalContext.sctx 的地址, 也是该结构体的起始地址
	csrr t0, sscratch
	// 保存 sp 到 LocalContext.sctx
	sd   sp, (t0)
	// 设置 sp 位 LocalContext 起始地址
	mv   sp, t0
	// 恢复线程上下文
	LOAD_ALL
	// LocalContext 的第三个值 LocalContext.x[1] 即 x2/sp
	ld   sp, 2*8(sp)
	// 执行线程
	sret

	// 从线程上下文恢复至调度上下文, sscratch 保存的值为 LocalContext 首地址
	// 陷入
	// 交换 sp 与 sscratch, sscratch 为 LocalContext 起始地址, sscratch临时保存 sp 
	.align 2        
	1:csrrw sp, sscratch, sp
	// 保存线程上下文
	SAVE_ALL
	// 将 sscratch 中的 sp 保存到 t0, 恢复 sscratch 为原 LocalContext 起始地址
	csrrw t0, sscratch, sp
	// 保存 sp 到 LocalContext.x[1], 即 x2/sp
	sd    t0, 2*8(sp)
	// sp 为 LocalContext 首地址, (sp) 为 LocalContext.sctx 保存调度上下文的 sp
	ld sp, (sp)
	// 恢复调度上下文
	LOAD_ALL
	addi sp, sp, 32*8
	// 返回调度
	ret

	/// 模拟异常发生, 将 2f 指令置为 sepc, 跳转至 1b, 返回时直接 ret
yield_naked:
	la t0, 2f
	csrw sepc, t0
	j 1b
	2:ret
