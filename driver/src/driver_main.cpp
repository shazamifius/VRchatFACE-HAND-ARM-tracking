// VRChat Bridge Hub - virtual HMD SteamVR driver ("vrcbridge").
//
// WHY THIS EXISTS
// ---------------
// VRChat only animates full body / trackers in VR mode, and VR mode requires
// SteamVR to report a present, tracked HMD. The stock "null" HMD starts SteamVR
// headless but renders nowhere, so the user sees nothing and cannot play. This
// driver instead exposes a *windowed* virtual HMD: IsDisplayOnDesktop() == true
// tells SteamVR's compositor to draw the stereo output into a normal desktop
// window region, so the user can actually see and play VRChat on their monitor
// with no physical headset. Body trackers come from our VMT path; head rotation
// will be fed into this driver from the Rust app in a later step (Milestone 1).
//
// MILESTONE 0 SCOPE (this file): a static, identity-pose HMD whose only job is
// to make SteamVR enter "HMD present" and give VRChat a viewable VR window. No
// networking yet — that keeps the proof minimal. The pose plumbing (GetPose /
// RunFrame -> TrackedDevicePoseUpdated) is already in place so head tracking is
// a small follow-up.
//
// A SteamVR driver is self-contained: it only needs openvr_driver.h, implements
// the device interfaces, and exports a single C factory `HmdDriverFactory`. It
// does NOT link against openvr_api. This is why it can live as a tiny C++ DLL
// bundled inside our otherwise-Rust app.

#include "openvr_driver.h"

#include <cstring>
#include <cstdint>
#include <atomic>
#include <mutex>
#include <thread>

#include <winsock2.h>
#include <ws2tcpip.h>
#pragma comment( lib, "ws2_32.lib" )

using namespace vr;

// UDP port the driver listens on for head orientation/position from the Rust
// "brain". The brain fuses webcam head tracking + mouse-look + (later) keyboard
// and sends the final head pose here. Body trackers still go to VMT (39570);
// this is the separate head channel our own driver owns.
static const unsigned short kHeadPosePort = 39571;

// Wire packet (little-endian floats): orientation as a quaternion (x,y,z,w) to
// match the rest of the app's xyzw convention, then position in metres (y up).
struct HeadPosePacket
{
	float qx, qy, qz, qw;
	float px, py, pz;
};

// UDP port for controller input from the Rust brain. The brain maps the mouse
// (aim + click) and keyboard (movement / menu) onto these two virtual hands so
// the user can navigate VRChat's VR menus and move — reproducing the desktop
// controls through emulated VR controllers.
static const unsigned short kControllerInputPort = 39572;

// One packet per hand. Packed (no padding) so the Rust side can lay the bytes
// out by hand and both ends agree on the 45-byte layout.
#pragma pack( push, 1 )
struct ControllerInputPacket
{
	uint8_t  hand;            // 0 = left, 1 = right
	float    px, py, pz;      // position (metres, y up)
	float    qx, qy, qz, qw;  // orientation (xyzw)
	uint32_t buttons;         // bitfield (see ControllerButton)
	float    trigger;         // analog trigger 0..1
	float    thumbX, thumbY;  // thumbstick -1..1
};
#pragma pack( pop )

// Button bits inside ControllerInputPacket::buttons. Must match the Rust sender.
enum ControllerButton : uint32_t
{
	kBtnTrigger    = 1u << 0,
	kBtnA          = 1u << 1,
	kBtnB          = 1u << 2,
	kBtnSystem     = 1u << 3,
	kBtnThumbstick = 1u << 4,
	kBtnGrip       = 1u << 5,
};

#if defined( _WIN32 )
#define HMD_DLL_EXPORT extern "C" __declspec( dllexport )
#else
#define HMD_DLL_EXPORT extern "C" __attribute__( ( visibility( "default" ) ) )
#endif

// ---------------------------------------------------------------------------
// Display geometry. Windowed-on-desktop "headset": the compositor renders into
// a kWindowWidth x kWindowHeight region at (kWindowX, kWindowY) on the monitor,
// split left/right into two eye viewports. kRender* is the per-eye render
// target SteamVR asks the app to draw.
// ---------------------------------------------------------------------------
static const int32_t  kWindowX = 0;
static const int32_t  kWindowY = 0;
static const uint32_t kWindowWidth = 1920;
static const uint32_t kWindowHeight = 1080;
static const uint32_t kRenderWidth = 960;  // per eye
static const uint32_t kRenderHeight = 1080;
static const float    kDisplayFrequency = 90.0f;
static const float    kIpdMeters = 0.063f;

// ===========================================================================
// The virtual HMD device. Implements both the tracked-device interface and the
// display component (returned via GetComponent).
// ===========================================================================
class CVrcBridgeHmd : public ITrackedDeviceServerDriver, public IVRDisplayComponent
{
public:
	CVrcBridgeHmd()
		: m_objectId( k_unTrackedDeviceIndexInvalid )
		, m_propContainer( k_ulInvalidPropertyContainer )
	{
	}

	// ---- ITrackedDeviceServerDriver ----

	EVRInitError Activate( uint32_t unObjectId ) override
	{
		m_objectId = unObjectId;
		m_propContainer = VRProperties()->TrackedDeviceToPropertyContainer( m_objectId );

		VRProperties()->SetStringProperty( m_propContainer, Prop_ModelNumber_String, "VRCBridge Virtual HMD" );
		VRProperties()->SetStringProperty( m_propContainer, Prop_RenderModelName_String, "generic_hmd" );
		VRProperties()->SetFloatProperty( m_propContainer, Prop_UserIpdMeters_Float, kIpdMeters );
		VRProperties()->SetFloatProperty( m_propContainer, Prop_UserHeadToEyeDepthMeters_Float, 0.f );
		VRProperties()->SetFloatProperty( m_propContainer, Prop_DisplayFrequency_Float, kDisplayFrequency );
		VRProperties()->SetFloatProperty( m_propContainer, Prop_SecondsFromVsyncToPhotons_Float, 0.011f );
		VRProperties()->SetUint64Property( m_propContainer, Prop_CurrentUniverseId_Uint64, 2 );
		// Not a normal desktop monitor; render in the compositor's window.
		VRProperties()->SetBoolProperty( m_propContainer, Prop_IsOnDesktop_Bool, false );
		VRProperties()->SetBoolProperty( m_propContainer, Prop_DisplayDebugMode_Bool, true );

		// Proximity sensor: we report the headset as permanently "worn" so
		// SteamVR never drops the HMD to standby. Without this, SteamVR sleeps
		// the static HMD and VRChat treats it as "headset removed" and
		// disconnects / pauses the session.
		VRDriverInput()->CreateBooleanComponent( m_propContainer, "/proximity", &m_proximity );
		VRDriverInput()->UpdateBooleanComponent( m_proximity, true, 0.0 );

		StartHeadListener();
		return VRInitError_None;
	}

	void Deactivate() override
	{
		StopHeadListener();
		m_objectId = k_unTrackedDeviceIndexInvalid;
	}
	void EnterStandby() override {}

	void *GetComponent( const char *pchComponentNameAndVersion ) override
	{
		if ( !std::strcmp( pchComponentNameAndVersion, IVRDisplayComponent_Version ) )
			return static_cast<IVRDisplayComponent *>( this );
		return nullptr;
	}

	void DebugRequest( const char *, char *pchResponseBuffer, uint32_t unResponseBufferSize ) override
	{
		if ( unResponseBufferSize >= 1 )
			pchResponseBuffer[0] = 0;
	}

	DriverPose_t GetPose() override
	{
		DriverPose_t pose = { 0 };
		pose.poseIsValid = true;
		pose.result = TrackingResult_Running_OK;
		pose.deviceIsConnected = true;

		// Identity transforms; world == driver space for now.
		pose.qWorldFromDriverRotation = { 1, 0, 0, 0 };
		pose.qDriverFromHeadRotation = { 1, 0, 0, 0 };

		// Head pose comes from the Rust brain over UDP (webcam + mouse-look),
		// defaulting to identity/eye-height until the first packet arrives.
		{
			std::lock_guard<std::mutex> lk( m_poseMutex );
			pose.qRotation = m_orientation;
			pose.vecPosition[0] = m_position[0];
			pose.vecPosition[1] = m_position[1];
			pose.vecPosition[2] = m_position[2];
		}

		return pose;
	}

	// Push the current pose to the runtime; called once per server frame.
	void RunFrame()
	{
		if ( m_objectId == k_unTrackedDeviceIndexInvalid )
			return;
		VRServerDriverHost()->TrackedDevicePoseUpdated( m_objectId, GetPose(), sizeof( DriverPose_t ) );
		// Keep asserting "worn" so SteamVR never enters standby.
		if ( m_proximity != k_ulInvalidInputComponentHandle )
			VRDriverInput()->UpdateBooleanComponent( m_proximity, true, 0.0 );
	}

	// ---- IVRDisplayComponent ----

	void GetWindowBounds( int32_t *pnX, int32_t *pnY, uint32_t *pnWidth, uint32_t *pnHeight ) override
	{
		*pnX = kWindowX;
		*pnY = kWindowY;
		*pnWidth = kWindowWidth;
		*pnHeight = kWindowHeight;
	}

	// true => SteamVR composites into the desktop window above (so the user
	// sees the game on their monitor instead of a real headset panel).
	bool IsDisplayOnDesktop() override { return true; }
	bool IsDisplayRealDisplay() override { return false; }

	void GetRecommendedRenderTargetSize( uint32_t *pnWidth, uint32_t *pnHeight ) override
	{
		*pnWidth = kRenderWidth;
		*pnHeight = kRenderHeight;
	}

	void GetEyeOutputViewport( EVREye eEye, uint32_t *pnX, uint32_t *pnY, uint32_t *pnWidth, uint32_t *pnHeight ) override
	{
		// MONO collapse: map BOTH eyes to the full window so the user sees ONE
		// flat image instead of the side-by-side stereo "double vision". The
		// compositor blits left then right into the same region, so the window
		// ends up showing a single eye's view full-screen — a normal PC-like
		// flat view, which is what desktop VRChat players want.
		(void)eEye;
		*pnX = 0;
		*pnY = 0;
		*pnWidth = kWindowWidth;
		*pnHeight = kWindowHeight;
	}

	void GetProjectionRaw( EVREye, float *pfLeft, float *pfRight, float *pfTop, float *pfBottom ) override
	{
		// Symmetric ~90 deg FOV (tangent half-angles).
		*pfLeft = -1.0f;
		*pfRight = 1.0f;
		*pfTop = -1.0f;
		*pfBottom = 1.0f;
	}

	DistortionCoordinates_t ComputeDistortion( EVREye, float fU, float fV ) override
	{
		// No lens distortion for a flat window.
		DistortionCoordinates_t coords;
		coords.rfRed[0] = fU; coords.rfRed[1] = fV;
		coords.rfGreen[0] = fU; coords.rfGreen[1] = fV;
		coords.rfBlue[0] = fU; coords.rfBlue[1] = fV;
		return coords;
	}

	bool ComputeInverseDistortion( HmdVector2_t *, EVREye, uint32_t, float, float ) override
	{
		return false;
	}

private:
	// Background UDP listener: receives the fused head pose from the Rust brain
	// and updates the shared orientation/position that GetPose reports.
	void StartHeadListener()
	{
		if ( m_netRunning.exchange( true ) )
			return; // already running

		m_netThread = std::thread( [this]() {
			WSADATA wsa;
			if ( WSAStartup( MAKEWORD( 2, 2 ), &wsa ) != 0 )
				return;

			SOCKET sock = socket( AF_INET, SOCK_DGRAM, IPPROTO_UDP );
			if ( sock == INVALID_SOCKET )
			{
				WSACleanup();
				return;
			}

			sockaddr_in addr {};
			addr.sin_family = AF_INET;
			addr.sin_port = htons( kHeadPosePort );
			inet_pton( AF_INET, "127.0.0.1", &addr.sin_addr );
			bind( sock, reinterpret_cast<sockaddr *>( &addr ), sizeof( addr ) );

			// Recv timeout so the thread can notice m_netRunning going false.
			DWORD timeoutMs = 200;
			setsockopt( sock, SOL_SOCKET, SO_RCVTIMEO, reinterpret_cast<char *>( &timeoutMs ), sizeof( timeoutMs ) );

			HeadPosePacket pkt;
			while ( m_netRunning )
			{
				int n = recv( sock, reinterpret_cast<char *>( &pkt ), sizeof( pkt ), 0 );
				if ( n == static_cast<int>( sizeof( pkt ) ) )
				{
					std::lock_guard<std::mutex> lk( m_poseMutex );
					// HmdQuaternion_t is { w, x, y, z }; packet is x,y,z,w.
					m_orientation = { pkt.qw, pkt.qx, pkt.qy, pkt.qz };
					m_position[0] = pkt.px;
					m_position[1] = pkt.py;
					m_position[2] = pkt.pz;
				}
			}

			closesocket( sock );
			WSACleanup();
		} );
	}

	void StopHeadListener()
	{
		m_netRunning = false;
		if ( m_netThread.joinable() )
			m_netThread.join();
	}

	uint32_t m_objectId;
	PropertyContainerHandle_t m_propContainer;
	VRInputComponentHandle_t m_proximity = k_ulInvalidInputComponentHandle;

	std::mutex m_poseMutex;
	HmdQuaternion_t m_orientation = { 1, 0, 0, 0 }; // { w, x, y, z }
	double m_position[3] = { 0.0, 1.6, 0.0 };       // metres, y up

	std::atomic<bool> m_netRunning { false };
	std::thread m_netThread;
};

// ===========================================================================
// A virtual hand controller. It MIMICS a Valve Index ("knuckles") controller:
// by reporting ControllerType "knuckles" and pointing the input profile at
// SteamVR's own bundled Index profile, VRChat auto-applies its existing knuckles
// bindings — so trigger/thumbstick/menu work with NO custom binding JSON from
// us. Pose + button/axis state are fed from the Rust brain over UDP (39572);
// until the first packet arrives it sits at a resting pose so the laser shows.
// ===========================================================================
class CVrcBridgeController : public ITrackedDeviceServerDriver
{
public:
	explicit CVrcBridgeController( ETrackedControllerRole role )
		: m_role( role )
	{
		// Resting pose: hand slightly to the side, below eye level, identity rot.
		m_pos[0] = ( role == TrackedControllerRole_LeftHand ) ? -0.2 : 0.2;
		m_pos[1] = 1.1;
		m_pos[2] = -0.3;
	}

	EVRInitError Activate( uint32_t unObjectId ) override
	{
		m_objectId = unObjectId;
		m_propContainer = VRProperties()->TrackedDeviceToPropertyContainer( m_objectId );

		VRProperties()->SetStringProperty( m_propContainer, Prop_TrackingSystemName_String, "vrcbridge" );
		VRProperties()->SetStringProperty( m_propContainer, Prop_ManufacturerName_String, "Valve" );
		VRProperties()->SetStringProperty( m_propContainer, Prop_ControllerType_String, "knuckles" );
		VRProperties()->SetStringProperty( m_propContainer, Prop_InputProfilePath_String,
			"{indexcontroller}/input/index_controller_profile.json" );
		VRProperties()->SetInt32Property( m_propContainer, Prop_ControllerRoleHint_Int32, m_role );
		VRProperties()->SetUint64Property( m_propContainer, Prop_CurrentUniverseId_Uint64, 2 );
		VRProperties()->SetBoolProperty( m_propContainer, Prop_DeviceIsWireless_Bool, true );

		if ( m_role == TrackedControllerRole_LeftHand )
		{
			VRProperties()->SetStringProperty( m_propContainer, Prop_ModelNumber_String, "Knuckles Left" );
			VRProperties()->SetStringProperty( m_propContainer, Prop_RenderModelName_String, "{indexcontroller}valve_controller_knu_1_0_left" );
			VRProperties()->SetStringProperty( m_propContainer, Prop_RegisteredDeviceType_String, "vrcbridge/controller_left" );
		}
		else
		{
			VRProperties()->SetStringProperty( m_propContainer, Prop_ModelNumber_String, "Knuckles Right" );
			VRProperties()->SetStringProperty( m_propContainer, Prop_RenderModelName_String, "{indexcontroller}valve_controller_knu_1_0_right" );
			VRProperties()->SetStringProperty( m_propContainer, Prop_RegisteredDeviceType_String, "vrcbridge/controller_right" );
		}

		auto *in = VRDriverInput();
		in->CreateBooleanComponent( m_propContainer, "/input/system/click", &m_hSystemClick );
		in->CreateBooleanComponent( m_propContainer, "/input/trigger/click", &m_hTriggerClick );
		in->CreateScalarComponent( m_propContainer, "/input/trigger/value", &m_hTriggerValue, VRScalarType_Absolute, VRScalarUnits_NormalizedOneSided );
		in->CreateBooleanComponent( m_propContainer, "/input/grip/click", &m_hGripClick );
		in->CreateScalarComponent( m_propContainer, "/input/grip/value", &m_hGripValue, VRScalarType_Absolute, VRScalarUnits_NormalizedOneSided );
		in->CreateScalarComponent( m_propContainer, "/input/grip/force", &m_hGripForce, VRScalarType_Absolute, VRScalarUnits_NormalizedOneSided );
		in->CreateScalarComponent( m_propContainer, "/input/thumbstick/x", &m_hThumbX, VRScalarType_Absolute, VRScalarUnits_NormalizedTwoSided );
		in->CreateScalarComponent( m_propContainer, "/input/thumbstick/y", &m_hThumbY, VRScalarType_Absolute, VRScalarUnits_NormalizedTwoSided );
		in->CreateBooleanComponent( m_propContainer, "/input/thumbstick/click", &m_hThumbClick );
		in->CreateBooleanComponent( m_propContainer, "/input/thumbstick/touch", &m_hThumbTouch );
		in->CreateBooleanComponent( m_propContainer, "/input/a/click", &m_hAClick );
		in->CreateBooleanComponent( m_propContainer, "/input/a/touch", &m_hATouch );
		in->CreateBooleanComponent( m_propContainer, "/input/b/click", &m_hBClick );
		in->CreateBooleanComponent( m_propContainer, "/input/b/touch", &m_hBTouch );
		in->CreateScalarComponent( m_propContainer, "/input/trackpad/x", &m_hTrackpadX, VRScalarType_Absolute, VRScalarUnits_NormalizedTwoSided );
		in->CreateScalarComponent( m_propContainer, "/input/trackpad/y", &m_hTrackpadY, VRScalarType_Absolute, VRScalarUnits_NormalizedTwoSided );
		in->CreateBooleanComponent( m_propContainer, "/input/trackpad/touch", &m_hTrackpadTouch );
		in->CreateScalarComponent( m_propContainer, "/input/finger/index", &m_hFingerIndex, VRScalarType_Absolute, VRScalarUnits_NormalizedOneSided );
		in->CreateScalarComponent( m_propContainer, "/input/finger/middle", &m_hFingerMiddle, VRScalarType_Absolute, VRScalarUnits_NormalizedOneSided );
		in->CreateScalarComponent( m_propContainer, "/input/finger/ring", &m_hFingerRing, VRScalarType_Absolute, VRScalarUnits_NormalizedOneSided );
		in->CreateScalarComponent( m_propContainer, "/input/finger/pinky", &m_hFingerPinky, VRScalarType_Absolute, VRScalarUnits_NormalizedOneSided );
		in->CreateHapticComponent( m_propContainer, "/output/haptic", &m_hHaptic );

		return VRInitError_None;
	}

	void Deactivate() override { m_objectId = k_unTrackedDeviceIndexInvalid; }
	void EnterStandby() override {}
	void *GetComponent( const char * ) override { return nullptr; }
	void DebugRequest( const char *, char *pchResponseBuffer, uint32_t unResponseBufferSize ) override
	{
		if ( unResponseBufferSize >= 1 )
			pchResponseBuffer[0] = 0;
	}

	DriverPose_t GetPose() override
	{
		DriverPose_t pose = { 0 };
		pose.poseIsValid = true;
		pose.result = TrackingResult_Running_OK;
		pose.deviceIsConnected = true;
		pose.qWorldFromDriverRotation = { 1, 0, 0, 0 };
		pose.qDriverFromHeadRotation = { 1, 0, 0, 0 };
		std::lock_guard<std::mutex> lk( m_mutex );
		pose.qRotation = m_rot;
		pose.vecPosition[0] = m_pos[0];
		pose.vecPosition[1] = m_pos[1];
		pose.vecPosition[2] = m_pos[2];
		return pose;
	}

	// Latch a freshly received input packet (called from the listener thread).
	void ApplyPacket( const ControllerInputPacket &pkt )
	{
		std::lock_guard<std::mutex> lk( m_mutex );
		m_rot = { pkt.qw, pkt.qx, pkt.qy, pkt.qz };
		m_pos[0] = pkt.px;
		m_pos[1] = pkt.py;
		m_pos[2] = pkt.pz;
		m_buttons = pkt.buttons;
		m_trigger = pkt.trigger;
		m_thumbX = pkt.thumbX;
		m_thumbY = pkt.thumbY;
	}

	void RunFrame()
	{
		if ( m_objectId == k_unTrackedDeviceIndexInvalid )
			return;
		VRServerDriverHost()->TrackedDevicePoseUpdated( m_objectId, GetPose(), sizeof( DriverPose_t ) );

		uint32_t buttons;
		float trigger, thumbX, thumbY;
		{
			std::lock_guard<std::mutex> lk( m_mutex );
			buttons = m_buttons;
			trigger = m_trigger;
			thumbX = m_thumbX;
			thumbY = m_thumbY;
		}

		auto *in = VRDriverInput();
		in->UpdateBooleanComponent( m_hTriggerClick, ( buttons & kBtnTrigger ) != 0, 0.0 );
		in->UpdateScalarComponent( m_hTriggerValue, ( buttons & kBtnTrigger ) ? 1.0f : trigger, 0.0 );
		in->UpdateBooleanComponent( m_hAClick, ( buttons & kBtnA ) != 0, 0.0 );
		in->UpdateBooleanComponent( m_hBClick, ( buttons & kBtnB ) != 0, 0.0 );
		in->UpdateBooleanComponent( m_hSystemClick, ( buttons & kBtnSystem ) != 0, 0.0 );
		in->UpdateBooleanComponent( m_hThumbClick, ( buttons & kBtnThumbstick ) != 0, 0.0 );
		in->UpdateBooleanComponent( m_hGripClick, ( buttons & kBtnGrip ) != 0, 0.0 );
		in->UpdateScalarComponent( m_hGripValue, ( buttons & kBtnGrip ) ? 1.0f : 0.0f, 0.0 );
		in->UpdateScalarComponent( m_hThumbX, thumbX, 0.0 );
		in->UpdateScalarComponent( m_hThumbY, thumbY, 0.0 );
		in->UpdateBooleanComponent( m_hThumbTouch, ( thumbX != 0.0f || thumbY != 0.0f ), 0.0 );
	}

private:
	ETrackedControllerRole m_role;
	uint32_t m_objectId = k_unTrackedDeviceIndexInvalid;
	PropertyContainerHandle_t m_propContainer = k_ulInvalidPropertyContainer;

	std::mutex m_mutex;
	HmdQuaternion_t m_rot = { 1, 0, 0, 0 }; // { w, x, y, z }
	double m_pos[3] = { 0.0, 1.1, -0.3 };
	uint32_t m_buttons = 0;
	float m_trigger = 0.f, m_thumbX = 0.f, m_thumbY = 0.f;

	VRInputComponentHandle_t m_hSystemClick = k_ulInvalidInputComponentHandle;
	VRInputComponentHandle_t m_hTriggerClick = k_ulInvalidInputComponentHandle;
	VRInputComponentHandle_t m_hTriggerValue = k_ulInvalidInputComponentHandle;
	VRInputComponentHandle_t m_hGripClick = k_ulInvalidInputComponentHandle;
	VRInputComponentHandle_t m_hGripValue = k_ulInvalidInputComponentHandle;
	VRInputComponentHandle_t m_hGripForce = k_ulInvalidInputComponentHandle;
	VRInputComponentHandle_t m_hThumbX = k_ulInvalidInputComponentHandle;
	VRInputComponentHandle_t m_hThumbY = k_ulInvalidInputComponentHandle;
	VRInputComponentHandle_t m_hThumbClick = k_ulInvalidInputComponentHandle;
	VRInputComponentHandle_t m_hThumbTouch = k_ulInvalidInputComponentHandle;
	VRInputComponentHandle_t m_hAClick = k_ulInvalidInputComponentHandle;
	VRInputComponentHandle_t m_hATouch = k_ulInvalidInputComponentHandle;
	VRInputComponentHandle_t m_hBClick = k_ulInvalidInputComponentHandle;
	VRInputComponentHandle_t m_hBTouch = k_ulInvalidInputComponentHandle;
	VRInputComponentHandle_t m_hTrackpadX = k_ulInvalidInputComponentHandle;
	VRInputComponentHandle_t m_hTrackpadY = k_ulInvalidInputComponentHandle;
	VRInputComponentHandle_t m_hTrackpadTouch = k_ulInvalidInputComponentHandle;
	VRInputComponentHandle_t m_hFingerIndex = k_ulInvalidInputComponentHandle;
	VRInputComponentHandle_t m_hFingerMiddle = k_ulInvalidInputComponentHandle;
	VRInputComponentHandle_t m_hFingerRing = k_ulInvalidInputComponentHandle;
	VRInputComponentHandle_t m_hFingerPinky = k_ulInvalidInputComponentHandle;
	VRInputComponentHandle_t m_hHaptic = k_ulInvalidInputComponentHandle;
};

// ===========================================================================
// The server provider: SteamVR's entry point into the driver. Registers the
// single virtual HMD and pumps its pose each frame.
// ===========================================================================
class CServerDriver : public IServerTrackedDeviceProvider
{
public:
	EVRInitError Init( IVRDriverContext *pDriverContext ) override
	{
		VR_INIT_SERVER_DRIVER_CONTEXT( pDriverContext );

		m_hmd = new CVrcBridgeHmd();
		VRServerDriverHost()->TrackedDeviceAdded( "VRCBRIDGE_HMD_0", TrackedDeviceClass_HMD, m_hmd );

		m_left = new CVrcBridgeController( TrackedControllerRole_LeftHand );
		VRServerDriverHost()->TrackedDeviceAdded( "VRCBRIDGE_CTRL_L", TrackedDeviceClass_Controller, m_left );
		m_right = new CVrcBridgeController( TrackedControllerRole_RightHand );
		VRServerDriverHost()->TrackedDeviceAdded( "VRCBRIDGE_CTRL_R", TrackedDeviceClass_Controller, m_right );

		StartControllerListener();
		return VRInitError_None;
	}

	void Cleanup() override
	{
		StopControllerListener();
		delete m_hmd;
		m_hmd = nullptr;
		delete m_left;
		m_left = nullptr;
		delete m_right;
		m_right = nullptr;
		VR_CLEANUP_SERVER_DRIVER_CONTEXT();
	}

	const char *const *GetInterfaceVersions() override { return k_InterfaceVersions; }

	void RunFrame() override
	{
		if ( m_hmd )
			m_hmd->RunFrame();
		if ( m_left )
			m_left->RunFrame();
		if ( m_right )
			m_right->RunFrame();
	}

	bool ShouldBlockStandbyMode() override { return false; }
	void EnterStandby() override {}
	void LeaveStandby() override {}

private:
	// Background UDP listener for controller input from the Rust brain. One
	// socket on 39572 receives per-hand packets and dispatches each to the
	// matching controller by its hand byte.
	void StartControllerListener()
	{
		if ( m_ctrlRunning.exchange( true ) )
			return;

		m_ctrlThread = std::thread( [this]() {
			WSADATA wsa;
			if ( WSAStartup( MAKEWORD( 2, 2 ), &wsa ) != 0 )
				return;

			SOCKET sock = socket( AF_INET, SOCK_DGRAM, IPPROTO_UDP );
			if ( sock == INVALID_SOCKET )
			{
				WSACleanup();
				return;
			}

			sockaddr_in addr {};
			addr.sin_family = AF_INET;
			addr.sin_port = htons( kControllerInputPort );
			inet_pton( AF_INET, "127.0.0.1", &addr.sin_addr );
			bind( sock, reinterpret_cast<sockaddr *>( &addr ), sizeof( addr ) );

			DWORD timeoutMs = 200;
			setsockopt( sock, SOL_SOCKET, SO_RCVTIMEO, reinterpret_cast<char *>( &timeoutMs ), sizeof( timeoutMs ) );

			ControllerInputPacket pkt;
			while ( m_ctrlRunning )
			{
				int n = recv( sock, reinterpret_cast<char *>( &pkt ), sizeof( pkt ), 0 );
				if ( n == static_cast<int>( sizeof( pkt ) ) )
				{
					if ( pkt.hand == 0 && m_left )
						m_left->ApplyPacket( pkt );
					else if ( pkt.hand == 1 && m_right )
						m_right->ApplyPacket( pkt );
				}
			}

			closesocket( sock );
			WSACleanup();
		} );
	}

	void StopControllerListener()
	{
		m_ctrlRunning = false;
		if ( m_ctrlThread.joinable() )
			m_ctrlThread.join();
	}

	CVrcBridgeHmd *m_hmd = nullptr;
	CVrcBridgeController *m_left = nullptr;
	CVrcBridgeController *m_right = nullptr;

	std::atomic<bool> m_ctrlRunning { false };
	std::thread m_ctrlThread;
};

// ---------------------------------------------------------------------------
// Factory: the one exported symbol SteamVR looks up in the DLL.
// ---------------------------------------------------------------------------
static CServerDriver g_serverDriver;

HMD_DLL_EXPORT void *HmdDriverFactory( const char *pInterfaceName, int *pReturnCode )
{
	if ( !std::strcmp( IServerTrackedDeviceProvider_Version, pInterfaceName ) )
		return &g_serverDriver;

	if ( pReturnCode )
		*pReturnCode = VRInitError_Init_InterfaceNotFound;
	return nullptr;
}
