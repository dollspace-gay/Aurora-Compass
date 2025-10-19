import React, {useCallback, useEffect} from 'react'
import {
  AccessibilityInfo,
  Image as RNImage,
  StyleSheet,
  useColorScheme,
  View,
} from 'react-native'
import Animated, {
  Easing,
  interpolate,
  runOnJS,
  useAnimatedStyle,
  useSharedValue,
  withTiming,
} from 'react-native-reanimated'
import {useSafeAreaInsets} from 'react-native-safe-area-context'
import Svg, {Path, type SvgProps} from 'react-native-svg'
import {Image} from 'expo-image'
import * as SplashScreen from 'expo-splash-screen'

import {Logotype} from '#/view/icons/Logotype'
// @ts-ignore
import splashImagePointer from '../assets/splash.png'
// @ts-ignore
import darkSplashImagePointer from '../assets/splash-dark.png'
const splashImageUri = RNImage.resolveAssetSource(splashImagePointer).uri
const darkSplashImageUri = RNImage.resolveAssetSource(
  darkSplashImagePointer,
).uri

export const Logo = React.forwardRef(function LogoImpl(props: SvgProps, ref) {
  const width = 1000
  const height = width
  return (
    <Image
      source={require('../assets/logo.png')}
      accessibilityLabel="Logo"
      accessibilityIgnoresInvertColors
      style={[{width, height}, props.style]}
    />
  )
})

type Props = {
  isReady: boolean
}

export function Splash(props: React.PropsWithChildren<Props>) {
  'use no memo'
  const insets = useSafeAreaInsets()
  const intro = useSharedValue(0)
  const outroLogo = useSharedValue(0)
  const outroApp = useSharedValue(0)
  const outroAppOpacity = useSharedValue(0)
  const [isAnimationComplete, setIsAnimationComplete] = React.useState(false)
  const [isImageLoaded, setIsImageLoaded] = React.useState(false)
  const [isLayoutReady, setIsLayoutReady] = React.useState(false)
  const [reduceMotion, setReduceMotion] = React.useState<boolean | undefined>(
    false,
  )
  const isReady =
    props.isReady &&
    isImageLoaded &&
    isLayoutReady &&
    reduceMotion !== undefined

  const colorScheme = useColorScheme()
  const isDarkMode = colorScheme === 'dark'

  const logoAnimation = useAnimatedStyle(() => {
    return {
      transform: [
        {
          scale: interpolate(intro.get(), [0, 1], [0.8, 1], 'clamp'),
        },
        {
          scale: interpolate(
            outroLogo.get(),
            [0, 0.08, 1],
            [1, 0.8, 500],
            'clamp',
          ),
        },
      ],
      opacity: interpolate(intro.get(), [0, 1], [0, 1], 'clamp'),
    }
  })
  const bottomLogoAnimation = useAnimatedStyle(() => {
    return {
      opacity: interpolate(intro.get(), [0, 1], [0, 1], 'clamp'),
    }
  })
  const reducedLogoAnimation = useAnimatedStyle(() => {
    return {
      transform: [
        {
          scale: interpolate(intro.get(), [0, 1], [0.8, 1], 'clamp'),
        },
      ],
      opacity: interpolate(intro.get(), [0, 1], [0, 1], 'clamp'),
    }
  })

  const logoWrapperAnimation = useAnimatedStyle(() => {
    return {
      opacity: interpolate(
        outroAppOpacity.get(),
        [0, 0.1, 0.2, 1],
        [1, 1, 0, 0],
        'clamp',
      ),
    }
  })

  const appAnimation = useAnimatedStyle(() => {
    return {
      transform: [
        {
          scale: interpolate(outroApp.get(), [0, 1], [1.1, 1], 'clamp'),
        },
      ],
      opacity: interpolate(
        outroAppOpacity.get(),
        [0, 0.1, 0.2, 1],
        [0, 0, 1, 1],
        'clamp',
      ),
    }
  })

  const onFinish = useCallback(() => setIsAnimationComplete(true), [])
  const onLayout = useCallback(() => setIsLayoutReady(true), [])
  const onLoadEnd = useCallback(() => setIsImageLoaded(true), [])

  useEffect(() => {
    if (isReady) {
      SplashScreen.hideAsync()
        .then(() => {
          intro.set(() =>
            withTiming(
              1,
              {duration: 400, easing: Easing.out(Easing.cubic)},
              async () => {
                // set these values to check animation at specific point
                outroLogo.set(() =>
                  withTiming(
                    1,
                    {duration: 1200, easing: Easing.in(Easing.cubic)},
                    () => {
                      runOnJS(onFinish)()
                    },
                  ),
                )
                outroApp.set(() =>
                  withTiming(1, {
                    duration: 1200,
                    easing: Easing.inOut(Easing.cubic),
                  }),
                )
                outroAppOpacity.set(() =>
                  withTiming(1, {
                    duration: 1200,
                    easing: Easing.in(Easing.cubic),
                  }),
                )
              },
            ),
          )
        })
        .catch(() => {})
    }
  }, [onFinish, intro, outroLogo, outroApp, outroAppOpacity, isReady])

  useEffect(() => {
    AccessibilityInfo.isReduceMotionEnabled().then(setReduceMotion)
  }, [])

  const logoAnimations =
    reduceMotion === true ? reducedLogoAnimation : logoAnimation
  // special off-spec color for dark mode
  const logoBg = isDarkMode ? '#0F1824' : '#fff'

  return (
    <View style={{flex: 1}} onLayout={onLayout}>
      {!isAnimationComplete && (
        <View style={StyleSheet.absoluteFillObject}>
          <Image
            accessibilityIgnoresInvertColors
            onLoadEnd={onLoadEnd}
            source={{uri: isDarkMode ? darkSplashImageUri : splashImageUri}}
            style={StyleSheet.absoluteFillObject}
          />

          <Animated.View
            style={[
              bottomLogoAnimation,
              {
                position: 'absolute',
                bottom: insets.bottom + 40,
                left: 0,
                right: 0,
                alignItems: 'center',
                justifyContent: 'center',
                opacity: 0,
              },
            ]}>
            <Logotype fill="#fff" width={90} />
          </Animated.View>
        </View>
      )}

      {isReady && (
        <>
          <Animated.View style={[{flex: 1}, appAnimation]}>
            {props.children}
          </Animated.View>

          {!isAnimationComplete && (
            <Animated.View
              style={[
                StyleSheet.absoluteFillObject,
                logoWrapperAnimation,
                {
                  flex: 1,
                  justifyContent: 'center',
                  alignItems: 'center',
                  transform: [{translateY: -(insets.top / 2)}, {scale: 0.1}], // scale from 1000px to 100px
                },
              ]}>
              <Animated.View style={[logoAnimations]}>
                <Logo fill={logoBg} />
              </Animated.View>
            </Animated.View>
          )}
        </>
      )}
    </View>
  )
}
