import React from 'react'
import {StyleSheet, type TextProps} from 'react-native'
import Svg, {
  Defs,
  LinearGradient,
  Path,
  type PathProps,
  Stop,
  type SvgProps,
} from 'react-native-svg'
import {Image} from 'expo-image'

import {colors} from '#/lib/styles'
import {useKawaiiMode} from '#/state/preferences/kawaii'

const ratio = 57 / 64

type Props = {
  fill?: PathProps['fill']
  style?: TextProps['style']
} & Omit<SvgProps, 'style'>

export const Logo = React.forwardRef(function LogoImpl(props: Props, ref) {
  const styles = StyleSheet.flatten(props.style)
  // @ts-ignore it's fiiiiine
  const size = parseInt(props.width || 32)
  // Make logo 4x bigger
  const actualSize = size * 4

  // Use custom logo image instead of Bluesky butterfly
  return (
    <Image
      source={require('../../../assets/logo.png')}
      accessibilityLabel="Logo"
      accessibilityHint=""
      accessibilityIgnoresInvertColors
      style={[{height: actualSize, width: actualSize, aspectRatio: 1}, styles]}
    />
  )
})
