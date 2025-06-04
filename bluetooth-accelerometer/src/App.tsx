import React, { useEffect, useRef, useState } from 'react';
import { SafeAreaView, ScrollView, StatusBar, useColorScheme } from 'react-native';
import Bluetooth from './components/Bluetooth';
import Accelerometer from './components/Accelerometer';
import { BluetoothDevice } from 'react-native-bluetooth-classic';
import KeepAwake from 'react-native-keep-awake';
import Toast, { BaseToast, ErrorToast } from 'react-native-toast-message';
import './global.css';

// Constants
const TEXT_SEND_INTERVAL = 10;   // Interval (ms) to send accelerometer data over Bluetooth
const SAMPLE_INTERVAL =10;      // Interval (ms) for sampling accelerometer data

function App(): React.JSX.Element {
  // State for accelerometer and Bluetooth device
  const [accelerometerData, setAccelerometerData] = useState({ x: 0, y: 0, z: 0 });
  const [connectedDevice, setConnectedDevice] = useState<BluetoothDevice | null>(null);

  // Ref to keep latest accelerometer data accessible without re-rendering
  const latestAccelerometerData = useRef(accelerometerData);
  const sendDataIntervalRef = useRef<NodeJS.Timeout | null>(null);

  // Keep the app awake to prevent it from going to sleep during data transmission
  useEffect(() => {
    KeepAwake.activate();
    return () => {
      KeepAwake.deactivate(); 
    };
  }, []);

  // Update ref when accelerometer data changes
  useEffect(() => {
    latestAccelerometerData.current = accelerometerData;
  }, [accelerometerData]);

  // Send data on interval based on connection state
  useEffect(() => {
    if (!connectedDevice) return;

    const interval = setInterval(() => {
      sendText();
    }, TEXT_SEND_INTERVAL);

    return () => {
      clearInterval(interval);
    };
  }, [connectedDevice]);

  // Send latest accelerometer data to the connected device
  async function sendText() {
    if (!connectedDevice) {
      return;
    }
  
    if (!(await connectedDevice.isConnected())) {
      return;
    }
  
    try {
      const { x, y, z } = latestAccelerometerData.current;
      const textToSend = `X: ${x.toFixed(2)}, Y: ${y.toFixed(2)}, Z: ${z.toFixed(2)}\n`;
      await connectedDevice.write(textToSend);
    } catch (error) {
    }
  }

  return (
    <>
      <SafeAreaView className="bg-dark flex-1">
        {/* Status bar */}
        <StatusBar barStyle={'light-content'}/>

        {/* ScrollView to hold all UI components */}
        <ScrollView
          contentInsetAdjustmentBehavior="automatic"
          style={{ flex: 1 }}
          contentContainerStyle={{ flexGrow: 1 }}
        >
          {/* Bluetooth device discovery and connection */}
          <Bluetooth
            connectedDevice={connectedDevice}
            setConnectedDevice={setConnectedDevice}
          />

          {/* Accelerometer display and movement tracking */}
          <Accelerometer
            setAccelerometerData={setAccelerometerData}
            sampleInterval={SAMPLE_INTERVAL}
          />
        </ScrollView>
      </SafeAreaView>
      <Toast
        config={{
          info: (props) => (
            <BaseToast
              {...props}
              style={{ backgroundColor: '#090909' }}
              contentContainerStyle={{ paddingHorizontal: 15 }}
              text1Style={{ color: 'white', fontSize: 16, fontWeight: 'bold' }}
              text2Style={{ color: 'white', fontSize: 14 }}
            />
          ),
          error: (props) => (
            <ErrorToast
              {...props}
              style={{ backgroundColor: '#090909', borderLeftColor: 'red' }}
              contentContainerStyle={{ paddingHorizontal: 15 }}
              text1Style={{ color: 'white', fontSize: 16, fontWeight: 'bold' }}
              text2Style={{ color: 'white', fontSize: 14 }}
            />
          ),
        }}
      />
    </>
  );
}

export default App;
