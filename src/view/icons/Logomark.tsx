import {type SvgProps} from 'react-native-svg'
import {Image} from 'expo-image'

export function Logomark({
  fill,
  ...rest
}: {fill?: any} & SvgProps) {
  // @ts-ignore it's fiiiiine
  const size = parseInt(rest.width || 32)
  // Make logo 4x bigger
  const actualSize = size * 4

  return (
    <Image
      source={require('../../../assets/logo.png')}
      accessibilityLabel="Logo"
      accessibilityHint=""
      accessibilityIgnoresInvertColors
      style={{height: actualSize, width: actualSize, aspectRatio: 1}}
    />
  )
}
