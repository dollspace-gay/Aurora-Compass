import {type SvgProps} from 'react-native-svg'
import {Image} from 'expo-image'

export function Logotype({
  fill,
  ...rest
}: {fill?: any} & SvgProps) {
  // @ts-ignore it's fiiiiine
  const size = parseInt(rest.width || 32)

  // Use custom logo instead of "Bluesky" text
  return (
    <Image
      source={require('../../../assets/logo.png')}
      accessibilityLabel="Aurora Compass"
      accessibilityIgnoresInvertColors
      style={{width: size, height: size, aspectRatio: 1}}
    />
  )
}
