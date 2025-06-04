import React, { useEffect, useState } from 'react';
import {
  View,
  Text,
  Alert,
  Platform,
  PermissionsAndroid,
  ActivityIndicator,
  Pressable,
} from 'react-native';
import Toast from 'react-native-toast-message';
import RNBluetoothClassic, { BluetoothDevice } from 'react-native-bluetooth-classic';

export default function Bluetooth({
  connectedDevice,
  setConnectedDevice,
}: {
  connectedDevice: BluetoothDevice | null;
  setConnectedDevice: (device: BluetoothDevice | null) => void;
}) {
  const [devices, setDevices] = useState<BluetoothDevice[]>([]);
  const [discovering, setDiscovering] = useState(false);

  // Check device connection status every second
  useEffect(() => {
    const interval = setInterval(() => {
      testConnection();
    }, 500);

    return () => clearInterval(interval);
  }, []);

  // Test if the currently connected device is still connected
  async function testConnection() {
    if (connectedDevice) {
      try {
        if (!(await connectedDevice.isConnected())) {
          console.log('Device is not connected, setting connectedDevice to null');
          setConnectedDevice(null);
        }
      } catch (error) {
        console.error('Error checking connection status:', error);
        Alert.alert('Error', 'Failed to check connection status.');
      }
    }
  }

  // Request Bluetooth permissions (differs for Android API levels)
  async function requestBluetoothPermissions() {
    if (Platform.OS === 'android' && Platform.Version >= 31) {
      try {
        const granted = await PermissionsAndroid.requestMultiple([
          PermissionsAndroid.PERMISSIONS.BLUETOOTH_SCAN,
          PermissionsAndroid.PERMISSIONS.BLUETOOTH_CONNECT,
          PermissionsAndroid.PERMISSIONS.ACCESS_FINE_LOCATION,
        ]);

        return (
          granted['android.permission.BLUETOOTH_SCAN'] === PermissionsAndroid.RESULTS.GRANTED &&
          granted['android.permission.BLUETOOTH_CONNECT'] === PermissionsAndroid.RESULTS.GRANTED &&
          granted['android.permission.ACCESS_FINE_LOCATION'] === PermissionsAndroid.RESULTS.GRANTED
        );
      } catch (err) {
        console.error('Failed to request Bluetooth permissions:', err);
        return false;
      }
    } else {
      try {
        const granted = await PermissionsAndroid.request(
          PermissionsAndroid.PERMISSIONS.ACCESS_FINE_LOCATION,
        );
        return granted === PermissionsAndroid.RESULTS.GRANTED;
      } catch (err) {
        console.error('Failed to request location permissions:', err);
        return false;
      }
    }
  }

  // Start Bluetooth discovery
  async function discoverDevices() {
    const hasPermissions = await requestBluetoothPermissions();
    if (!hasPermissions) {
      Alert.alert('Error', 'Bluetooth permissions are required to discover devices.');
      return;
    }

    try {
      const available = await RNBluetoothClassic.isBluetoothAvailable();
      console.log('Bluetooth available:', available);
    } catch (error) {
      console.error('Error checking Bluetooth availability:', error);
      Alert.alert('Error', 'Could not check Bluetooth availability.\n' + error);
    }

    setDiscovering(true);
    setDevices([]);

    try {
      const availableDevices = await RNBluetoothClassic.startDiscovery();
      setDevices(availableDevices);

      Toast.show({
        type: 'info',
        text1: `Found ${availableDevices.length} devices`,
        text2: `${availableDevices.filter((device) => device.name && !device.name.includes(':')).length} available for connection`,
        visibilityTime: 1500, 
      });
    } catch (error) {
      Toast.show({
        type: 'error',
        text1: `Could not discover devices`,
        text2: `Enable Bluetooth first`,
        visibilityTime: 3000, 
      });
    } finally {
      setDiscovering(false);
    }
  }

  // Attempt to connect to a selected device
  async function connectToDevice(device: BluetoothDevice) {
    try {
      console.log(`Connecting to ${device.name}...`);
      const connected = await device.connect();

      console.log("Stabilizing connection...");
      // Wait briefly to allow native connection to stabilize
      await new Promise(resolve => setTimeout(resolve, 500));

      const isStillConnected = await device.isConnected();
      console.log(`Connection status for ${device.name}: ${isStillConnected}`);

      if (connected && isStillConnected) {
        console.log(`Successfully connected to ${device.name}`);
        setConnectedDevice(device);
        Toast.show({
          type: 'info',
          text1: `Connected to ${device.name}`,
          visibilityTime: 1500,
        });
      } else {
        Toast.show({
          type: 'error',
          text1: `Connection to ${device.name} failed`,
          text2: 'Device disconnected right after connect().',
          visibilityTime: 1500,
        });
      }
    } catch (error) {
      console.error('Error connecting to device:', error);
      Alert.alert('Error', `Could not connect to ${device.name}\n${error}`);
    }
  }

  // Disconnect from the current device
  async function disconnect() {
    if (connectedDevice) {
      try {
        console.log(`Disconnecting from ${connectedDevice.name}...`);
        setConnectedDevice(null);
        console.log('connectedDevice set to null');
        const disconnected = await connectedDevice.disconnect();
        console.log(`Disconnected from ${connectedDevice.name}: ${disconnected}`);
        if (disconnected) {
          Toast.show({
            type: 'info',
            text1: `Disconnected from ${connectedDevice.name}`,
            visibilityTime: 1500,
          });
        }
      } catch (error) {
        console.error('Error disconnecting from device:', error);
        Alert.alert('Error', `Could not disconnect from ${connectedDevice.name}.`);
        setConnectedDevice(null);
      }
    }
  }

  return (
    <View className="p-[28px] flex bg-dark">
      {/* Heading */}
      <Text className="text-[32px] text-white mb-[10px] font-bold">Bluetooth Devices</Text>

      {/* Currently connected device name */}
      <Text className="text-[16px] mb-[16px] text-white">
        Currently connected to: {connectedDevice ? connectedDevice.name : 'Nothing'}
      </Text>

      {/* List of discovered devices (filtered by name) */}
      {!connectedDevice && devices
        .filter((device) => device.name && !device.name.includes(':'))
        .map((device) => (
          <View key={device.id} className="flex-row items-center space-x-2 mt-[2px]">
            <View className="m-4 size-3 rounded-full bg-white" />
            <Pressable
              className="bg-[#5e5e5e] p-3 rounded-md w-full mt-[10px] flex-1"
              android_ripple={{ color: 'rgba(255, 255, 255, 0.2)' }}
              onPress={() => connectToDevice(device)}
            >
              <Text className="text-white text-center uppercase">{`Connect to ${device.name}`}</Text>
            </Pressable>
          </View>
        ))}

      {/* Discover devices button */}
      {!connectedDevice && !discovering && (
        <Pressable 
          className="bg-[#5e5e5e] p-3 rounded-md mt-[20px]" 
          android_ripple={{ color: 'rgba(255, 255, 255, 0.2)' }}
          onPress={discoverDevices}
        >
          <Text className="text-white text-center uppercase">Discover Devices</Text>
        </Pressable>
      )}

      {/* Loading indicator during discovery */}
      {discovering && 
        <ActivityIndicator className="mt-[12px]" size="large" color="#ffffff" />
      }

      {/* Disconnect button if a device is connected */}
      {connectedDevice && (
        <Pressable 
          className="bg-[#5e5e5e] p-3 rounded-md mt-[10px]" 
          android_ripple={{ color: 'rgba(255, 255, 255, 0.2)' }}
          onPress={disconnect}
        >
          <Text className="text-white text-center uppercase">Disconnect</Text>
        </Pressable>
      )}
    </View>
  );
}
