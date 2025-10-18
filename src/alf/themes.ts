import {
  createThemes,
  DEFAULT_PALETTE,
  DEFAULT_SUBDUED_PALETTE,
} from '@bsky.app/alf'

// Aurora Prism theme - pink/red/purple frequency
const CUSTOM_PALETTE = {
  ...DEFAULT_PALETTE,
  // Override primary colors to use red-purple gradient
  primary_25: '#fff0f5',   // Very light pink
  primary_50: '#ffe6f0',   // Light pink
  primary_100: '#ffcce0',  // Soft pink
  primary_200: '#ffb3d1',  // Light red-pink
  primary_300: '#ff99c2',  // Medium pink
  primary_400: '#ff66a3',  // Vibrant pink-red
  primary_500: '#ff3366',  // Primary red (main brand color)
  primary_600: '#e6194b',  // Rich red
  primary_700: '#b3003d',  // Deep red
  primary_800: '#80002e',  // Dark red
  primary_900: '#4d001a',  // Very dark red
  primary_950: '#1a0009',  // Almost black red

  // Aurora-inspired dark backgrounds with purple-pink glow
  contrast_0: '#0d0515',   // Deep purple-black (darkest) - aurora night sky
  contrast_25: '#1a0f24',  // Dark purple midnight
  contrast_50: '#251633',  // Dark purple-navy
  contrast_100: '#342047', // Purple-navy with pink undertones
  contrast_200: '#4a2d5c', // Purple-slate with aurora glow
}

const CUSTOM_SUBDUED_PALETTE = {
  ...DEFAULT_SUBDUED_PALETTE,
  // Subdued version with more purple tones
  primary_25: '#f5f0ff',   // Very light purple
  primary_50: '#ebe6ff',   // Light purple
  primary_100: '#d6ccff',  // Soft purple
  primary_200: '#c2b3ff',  // Light purple
  primary_300: '#ad99ff',  // Medium purple
  primary_400: '#8866ff',  // Vibrant purple
  primary_500: '#6d00fa',  // Primary purple
  primary_600: '#5500c7',  // Rich purple
  primary_700: '#420099',  // Deep purple
  primary_800: '#2e006b',  // Dark purple
  primary_900: '#1a003d',  // Very dark purple
  primary_950: '#0a0019',  // Almost black purple

  // Aurora-inspired dark backgrounds with deeper purple glow
  contrast_0: '#0f0a1a',   // Deep purple-black (darkest) - aurora night
  contrast_25: '#1a1428',  // Dark purple midnight
  contrast_50: '#251e36',  // Dark purple-navy
  contrast_100: '#322a47', // Purple-navy
  contrast_200: '#453d63', // Purple-slate with aurora shimmer
}

const DEFAULT_THEMES = createThemes({
  defaultPalette: CUSTOM_PALETTE,
  subduedPalette: CUSTOM_SUBDUED_PALETTE,
})

export const themes = {
  lightPalette: DEFAULT_THEMES.light.palette,
  darkPalette: DEFAULT_THEMES.dark.palette,
  dimPalette: DEFAULT_THEMES.dim.palette,
  light: DEFAULT_THEMES.light,
  dark: DEFAULT_THEMES.dark,
  dim: DEFAULT_THEMES.dim,
}

/**
 * @deprecated use ALF and access palette from `useTheme()`
 */
export const lightPalette = DEFAULT_THEMES.light.palette
/**
 * @deprecated use ALF and access palette from `useTheme()`
 */
export const darkPalette = DEFAULT_THEMES.dark.palette
/**
 * @deprecated use ALF and access palette from `useTheme()`
 */
export const dimPalette = DEFAULT_THEMES.dim.palette
/**
 * @deprecated use ALF and access theme from `useTheme()`
 */
export const light = DEFAULT_THEMES.light
/**
 * @deprecated use ALF and access theme from `useTheme()`
 */
export const dark = DEFAULT_THEMES.dark
/**
 * @deprecated use ALF and access theme from `useTheme()`
 */
export const dim = DEFAULT_THEMES.dim
