import React, { useState, useEffect, useRef } from 'react';
import { View, Text, Animated } from 'react-native';
import { SensorTypes, accelerometer, setUpdateIntervalForType } from 'react-native-sensors';

// Constants defining sizes of UI elements
const BOUNDARY_SIZE = 250;
const BALL_SIZE = 30;
const CENTER_INDICATOR_SIZE = 6;

// Alpha controls how smooth the filtering is (lower = smoother, slower)
const SMOOTHING_ALPHA = 0.06;

// Linear interpolation function for smoothing
const lerp = (a: number, b: number, t: number) => a + (b - a) * t;

export default function Accelerometer({
  setAccelerometerData,
  sampleInterval
}: {
  setAccelerometerData: (data: { x: number; y: number; z: number }) => void,
  sampleInterval: number
}) {
  // Displayed raw data (optional — could show smoothed instead)
  const [data, setData] = useState<{ x: number; y: number; z: number }>({ x: 0, y: 0, z: 0 });

  // Smoothed position for animation
  const position = useRef({
    x: new Animated.Value(0),
    y: new Animated.Value(0),
  }).current;

  // Target values for animation
  const target = useRef<{ x: number; y: number }>({ x: 0, y: 0 });

  // Store smoothed accelerometer data
  const smoothedData = useRef<{ x: number; y: number; z: number }>({ x: 0, y: 0, z: 0 });

  useEffect(() => {
    // Set accelerometer update interval (ms)
    setUpdateIntervalForType(SensorTypes.accelerometer, sampleInterval);
  
    // Subscribe to accelerometer stream
    const subscription = accelerometer.subscribe(({ x, y, z }) => {
  
      // Smooth the data using lerp
      smoothedData.current = {
        x: lerp(smoothedData.current.x, -x, SMOOTHING_ALPHA),
        y: lerp(smoothedData.current.y, -y, SMOOTHING_ALPHA),
        z: lerp(smoothedData.current.z, -z, SMOOTHING_ALPHA),
      };
  
      // Send smoothed data to parent (used for Bluetooth transmission)
      setAccelerometerData(smoothedData.current);
  
      // Optional: show raw (inverted) data in the UI
      setData({ x: -x, y: -y, z: -z });
  
      // Update animation target (based on raw values for snappier UI)
      target.current = {
        x: -x * (BOUNDARY_SIZE / 2 - BALL_SIZE / 2) / 10,
        y: y * (BOUNDARY_SIZE / 2 - BALL_SIZE / 2) / 10,
      };
    });

    // Animate ball position toward target in small smooth steps
    const interval = setInterval(() => {
      Animated.timing(position.x, {
        toValue: target.current.x,
        duration: 140,
        useNativeDriver: false,
      }).start();

      Animated.timing(position.y, {
        toValue: target.current.y,
        duration: 140,
        useNativeDriver: false,
      }).start();
    }, 40);

    return () => {
      subscription.unsubscribe();
      clearInterval(interval);
    };
  }, []);

  return (
    <View className="p-[28px] flex bg-dark">
      {/* Title */}
      <Text className="text-[32px] text-white mb-[10px] font-bold">Accelerometer</Text>

      {/* Display accelerometer values (raw for visibility) */}
      <View className="flex-row justify-between">
        <Text className="text-white text-[16px] mb-[10px]">X: {data.x.toFixed(1)}</Text>
        <Text className="text-white text-[16px] mb-[10px]">Y: {data.y.toFixed(1)}</Text>
        <Text className="text-white text-[16px] mb-[10px]">Z: {data.z.toFixed(1)}</Text>
      </View>

      {/* Ball visualizer area */}
      <View className="flex justify-center items-center mt-[24px]">
        <View
          style={{ width: BOUNDARY_SIZE, height: BOUNDARY_SIZE }}
          className="border-[1.5px] border-white justify-center items-center relative overflow-hidden rounded-lg"
        >
          {/* Static center dot for reference */}
          <Animated.View
            style={{
              width: CENTER_INDICATOR_SIZE,
              height: CENTER_INDICATOR_SIZE,
              borderRadius: 4,
            }}
            className="bg-white absolute"
          />

          {/* Movable ball representing tilt */}
          <Animated.View
            style={{
              width: BALL_SIZE,
              height: BALL_SIZE,
              borderRadius: BALL_SIZE / 2,
              transform: [{ translateX: position.x }, { translateY: position.y }],
            }}
            className="bg-white absolute"
          />
        </View>
      </View>
    </View>
  );
}
