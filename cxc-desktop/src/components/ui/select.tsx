"use client"

import * as React from "react"
import { Select as SelectPrimitive } from "radix-ui"
import { cn } from "@/lib/utils"
import { ChevronDownIcon, CheckIcon, ChevronUpIcon } from "lucide-react"

/**
 * 内部用于映射空字符串的特殊值（因为 Radix Select 不允许 Item 的 value 为空字符串）
 */
const EMPTY_SELECT_VALUE = "__CXC_EMPTY_VALUE__"

function Select({
  ...props
}: React.ComponentProps<typeof SelectPrimitive.Root>) {
  return <SelectPrimitive.Root data-slot="select" {...props} />
}

function SelectGroup({
  className,
  ...props
}: React.ComponentProps<typeof SelectPrimitive.Group>) {
  return (
    <SelectPrimitive.Group
      data-slot="select-group"
      className={cn("scroll-my-1 p-1", className)}
      {...props}
    />
  )
}

function SelectValue({
  ...props
}: React.ComponentProps<typeof SelectPrimitive.Value>) {
  return <SelectPrimitive.Value data-slot="select-value" {...props} />
}

function SelectTrigger({
  className,
  size = "default",
  children,
  ...props
}: React.ComponentProps<typeof SelectPrimitive.Trigger> & {
  size?: "default" | "sm" | "xs"
}) {
  return (
    <SelectPrimitive.Trigger
      data-slot="select-trigger"
      data-size={size}
      className={cn(
        "group/select-trigger flex w-full items-center justify-between gap-1.5 rounded-md border border-border/30 bg-muted/10 text-xs text-foreground shadow-inner transition-all duration-200 outline-none select-none cursor-pointer",
        "hover:bg-muted/15 hover:border-border/60",
        "focus-visible:border-primary/60 focus-visible:ring-1 focus-visible:ring-primary/40",
        "data-[state=open]:border-primary/60 data-[state=open]:ring-1 data-[state=open]:ring-primary/40 data-[state=open]:bg-background/80",
        "disabled:cursor-not-allowed disabled:opacity-50",
        "data-[placeholder]:text-muted-foreground/70",
        "data-[size=default]:h-9 data-[size=default]:px-2.5",
        "data-[size=sm]:h-8 data-[size=sm]:px-2.5",
        "data-[size=xs]:h-7.5 data-[size=xs]:px-2",
        "*:data-[slot=select-value]:line-clamp-1 *:data-[slot=select-value]:flex *:data-[slot=select-value]:items-center *:data-[slot=select-value]:gap-1.5",
        "[&_svg]:pointer-events-none [&_svg]:shrink-0",
        className
      )}
      {...props}
    >
      {children}
      <SelectPrimitive.Icon asChild>
        <ChevronDownIcon className="size-3.5 shrink-0 text-muted-foreground/60 transition-transform duration-200 group-data-[state=open]/select-trigger:rotate-180 group-data-[state=open]/select-trigger:text-primary" />
      </SelectPrimitive.Icon>
    </SelectPrimitive.Trigger>
  )
}

function SelectContent({
  className,
  children,
  position = "popper",
  align = "start",
  sideOffset = 4,
  ...props
}: React.ComponentProps<typeof SelectPrimitive.Content>) {
  return (
    <SelectPrimitive.Portal>
      <SelectPrimitive.Content
        data-slot="select-content"
        data-align-trigger={position === "item-aligned"}
        className={cn(
          "relative z-[100] max-h-72 min-w-[var(--radix-select-trigger-width)] overflow-hidden rounded-xl border border-border/60 bg-card/95 text-foreground shadow-2xl backdrop-blur-xl duration-150 outline-none",
          "data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95",
          "data-[side=bottom]:slide-in-from-top-1 data-[side=top]:slide-in-from-bottom-1 data-[side=left]:slide-in-from-right-1 data-[side=right]:slide-in-from-left-1",
          position === "popper" && "data-[side=bottom]:translate-y-1 data-[side=top]:-translate-y-1",
          className
        )}
        position={position}
        align={align}
        sideOffset={sideOffset}
        {...props}
      >
        <SelectScrollUpButton />
        <SelectPrimitive.Viewport
          data-position={position}
          className={cn(
            "p-1 max-h-60 overflow-y-auto scroll-smooth",
            "[&::-webkit-scrollbar]:w-1.5 [&::-webkit-scrollbar-track]:bg-transparent [&::-webkit-scrollbar-thumb]:bg-border/60 [&::-webkit-scrollbar-thumb]:rounded-full hover:[&::-webkit-scrollbar-thumb]:bg-border/90",
            position === "popper" && "w-full min-w-[var(--radix-select-trigger-width)]"
          )}
        >
          {children}
        </SelectPrimitive.Viewport>
        <SelectScrollDownButton />
      </SelectPrimitive.Content>
    </SelectPrimitive.Portal>
  )
}

function SelectLabel({
  className,
  ...props
}: React.ComponentProps<typeof SelectPrimitive.Label>) {
  return (
    <SelectPrimitive.Label
      data-slot="select-label"
      className={cn("px-2.5 py-1 text-[11px] font-extrabold uppercase tracking-wider text-muted-foreground/75", className)}
      {...props}
    />
  )
}

function SelectItem({
  className,
  children,
  ...props
}: React.ComponentProps<typeof SelectPrimitive.Item>) {
  return (
    <SelectPrimitive.Item
      data-slot="select-item"
      className={cn(
        "relative flex w-full cursor-pointer select-none items-center rounded-lg py-1.5 pl-2.5 pr-7 text-xs font-normal outline-none transition-colors",
        "text-foreground/90 hover:bg-muted/40 hover:text-foreground",
        "focus:bg-primary/10 focus:text-primary data-[highlighted]:bg-primary/10 data-[highlighted]:text-primary",
        "data-[state=checked]:font-semibold data-[state=checked]:text-primary",
        "data-disabled:pointer-events-none data-disabled:opacity-40",
        className
      )}
      {...props}
    >
      <SelectPrimitive.ItemText>{children}</SelectPrimitive.ItemText>
      <span className="pointer-events-none absolute right-2 flex size-3.5 items-center justify-center">
        <SelectPrimitive.ItemIndicator>
          <CheckIcon className="size-3.5 text-primary stroke-[2.5]" />
        </SelectPrimitive.ItemIndicator>
      </span>
    </SelectPrimitive.Item>
  )
}

function SelectSeparator({
  className,
  ...props
}: React.ComponentProps<typeof SelectPrimitive.Separator>) {
  return (
    <SelectPrimitive.Separator
      data-slot="select-separator"
      className={cn("pointer-events-none -mx-1 my-1 h-px bg-border/40", className)}
      {...props}
    />
  )
}

function SelectScrollUpButton({
  className,
  ...props
}: React.ComponentProps<typeof SelectPrimitive.ScrollUpButton>) {
  return (
    <SelectPrimitive.ScrollUpButton
      data-slot="select-scroll-up-button"
      className={cn(
        "z-10 flex cursor-default items-center justify-center bg-card/90 py-1 text-muted-foreground/70 hover:text-foreground transition-colors",
        className
      )}
      {...props}
    >
      <ChevronUpIcon className="size-3.5" />
    </SelectPrimitive.ScrollUpButton>
  )
}

function SelectScrollDownButton({
  className,
  ...props
}: React.ComponentProps<typeof SelectPrimitive.ScrollDownButton>) {
  return (
    <SelectPrimitive.ScrollDownButton
      data-slot="select-scroll-down-button"
      className={cn(
        "z-10 flex cursor-default items-center justify-center bg-card/90 py-1 text-muted-foreground/70 hover:text-foreground transition-colors",
        className
      )}
      {...props}
    >
      <ChevronDownIcon className="size-3.5" />
    </SelectPrimitive.ScrollDownButton>
  )
}

export interface SelectOptionItem {
  value: string
  label: React.ReactNode
  disabled?: boolean
}

export interface CxcSelectProps {
  id?: string
  value?: string
  defaultValue?: string
  onValueChange?: (value: string) => void
  options: (string | SelectOptionItem)[]
  placeholder?: string
  disabled?: boolean
  size?: "default" | "sm" | "xs"
  className?: string
  triggerClassName?: string
  contentClassName?: string
}

/**
 * 封装便捷的 CxcSelect 下拉选择组件：
 * 1. 自动处理空字符串与 Radix Select 的兼容映射；
 * 2. 自动保持 Notion 极简纸张质感与品牌蓝焦点；
 * 3. 支持 options 为纯字符串数组或 `{ value, label, disabled }` 对象数组；
 * 4. 自动保留不在 options 中的当前非空 value，防止值丢失或回显异常。
 */
function CxcSelect({
  id,
  value,
  defaultValue,
  onValueChange,
  options,
  placeholder,
  disabled,
  size = "default",
  triggerClassName,
  contentClassName,
}: CxcSelectProps) {
  const normalizedOptions: SelectOptionItem[] = React.useMemo(() => {
    const list = options.map((opt) =>
      typeof opt === "string" ? { value: opt, label: opt } : opt
    )
    if (value && !list.some((opt) => opt.value === value)) {
      list.push({ value, label: value })
    }
    return list
  }, [options, value])

  const internalValue = value === "" ? EMPTY_SELECT_VALUE : value
  const internalDefault = defaultValue === "" ? EMPTY_SELECT_VALUE : defaultValue

  const handleValueChange = (val: string) => {
    const next = val === EMPTY_SELECT_VALUE ? "" : val
    onValueChange?.(next)
  }

  return (
    <Select
      value={internalValue}
      defaultValue={internalDefault}
      onValueChange={handleValueChange}
      disabled={disabled}
    >
      <SelectTrigger id={id} size={size} className={triggerClassName}>
        <SelectValue placeholder={placeholder} />
      </SelectTrigger>
      <SelectContent className={contentClassName}>
        {normalizedOptions.map((opt) => (
          <SelectItem
            key={opt.value}
            value={opt.value === "" ? EMPTY_SELECT_VALUE : opt.value}
            disabled={opt.disabled}
          >
            {opt.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}

export {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectScrollDownButton,
  SelectScrollUpButton,
  SelectSeparator,
  SelectTrigger,
  SelectValue,
  CxcSelect,
  EMPTY_SELECT_VALUE,
}
