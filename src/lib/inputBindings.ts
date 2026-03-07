export type InputBindingKind = 'none' | 'keyboard' | 'mouse'

export interface InputBinding {
  kind: InputBindingKind
  value: string
}

export interface InputBindingsConfig {
  enabled: boolean
  hideActionBarWhenActive: boolean
  prevStep: InputBinding
  nextStep: InputBinding
}

export interface InputBindingsStatus {
  prevRegistered: boolean
  nextRegistered: boolean
  mouseSupported: boolean
  canHideActionBar: boolean
  actionBarHidden: boolean
  errors: string[]
}

export interface InputBindingOption {
  label: string
  kind: InputBindingKind
  value: string
}

const KEYBOARD_BINDING_OPTIONS: InputBindingOption[] = [
  ...Array.from({ length: 12 }, (_, index) => ({
    label: `F${index + 1}`,
    kind: 'keyboard' as const,
    value: `F${index + 1}`,
  })),
  ...'ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split('').map((letter) => ({
    label: letter,
    kind: 'keyboard' as const,
    value: `Key${letter}`,
  })),
  ...Array.from({ length: 10 }, (_, index) => ({
    label: `${index}`,
    kind: 'keyboard' as const,
    value: `Digit${index}`,
  })),
  ...Array.from({ length: 10 }, (_, index) => ({
    label: `小键盘 ${index}`,
    kind: 'keyboard' as const,
    value: `Numpad${index}`,
  })),
  { label: '空格', kind: 'keyboard', value: 'Space' },
  { label: 'Tab', kind: 'keyboard', value: 'Tab' },
  { label: '回车', kind: 'keyboard', value: 'Enter' },
  { label: '退格', kind: 'keyboard', value: 'Backspace' },
  { label: 'Esc', kind: 'keyboard', value: 'Escape' },
  { label: 'PageUp', kind: 'keyboard', value: 'PageUp' },
  { label: 'PageDown', kind: 'keyboard', value: 'PageDown' },
  { label: 'Home', kind: 'keyboard', value: 'Home' },
  { label: 'End', kind: 'keyboard', value: 'End' },
  { label: 'Insert', kind: 'keyboard', value: 'Insert' },
  { label: 'Delete', kind: 'keyboard', value: 'Delete' },
  { label: '上方向键', kind: 'keyboard', value: 'ArrowUp' },
  { label: '下方向键', kind: 'keyboard', value: 'ArrowDown' },
  { label: '左方向键', kind: 'keyboard', value: 'ArrowLeft' },
  { label: '右方向键', kind: 'keyboard', value: 'ArrowRight' },
]

export const INPUT_BINDING_OPTIONS: InputBindingOption[] = [
  { label: '未设置', kind: 'none', value: '' },
  { label: '鼠标侧键 1', kind: 'mouse', value: 'side1' },
  { label: '鼠标侧键 2', kind: 'mouse', value: 'side2' },
  ...KEYBOARD_BINDING_OPTIONS,
]

export function createDefaultInputBindings(): InputBindingsConfig {
  return {
    enabled: true,
    hideActionBarWhenActive: false,
    prevStep: { kind: 'mouse', value: 'side1' },
    nextStep: { kind: 'mouse', value: 'side2' },
  }
}

export function createEmptyInputBindingsStatus(): InputBindingsStatus {
  return {
    prevRegistered: false,
    nextRegistered: false,
    mouseSupported: false,
    canHideActionBar: false,
    actionBarHidden: false,
    errors: [],
  }
}

export function cloneInputBindings(config: InputBindingsConfig): InputBindingsConfig {
  return {
    enabled: config.enabled,
    hideActionBarWhenActive: config.hideActionBarWhenActive,
    prevStep: { ...config.prevStep },
    nextStep: { ...config.nextStep },
  }
}

export function toBindingOptionValue(binding: InputBinding): string {
  if (binding.kind === 'none' || !binding.value) {
    return 'none:'
  }

  return `${binding.kind}:${binding.value}`
}

export function fromBindingOptionValue(rawValue: string): InputBinding {
  const [kind, value = ''] = rawValue.split(':', 2)

  if (kind === 'keyboard' || kind === 'mouse') {
    return { kind, value }
  }

  return { kind: 'none', value: '' }
}
