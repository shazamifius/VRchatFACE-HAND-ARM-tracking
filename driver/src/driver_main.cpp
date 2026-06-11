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

using namespace vr;

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

		return VRInitError_None;
	}

	void Deactivate() override { m_objectId = k_unTrackedDeviceIndexInvalid; }
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

		// Static head at ~1.6 m eye height, looking forward. Head rotation will
		// be driven from the Rust app in Milestone 1 by updating m_orientation.
		pose.qRotation = m_orientation;
		pose.vecPosition[0] = 0.0;
		pose.vecPosition[1] = 1.6;
		pose.vecPosition[2] = 0.0;

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
		*pnY = 0;
		*pnWidth = kWindowWidth / 2;
		*pnHeight = kWindowHeight;
		*pnX = ( eEye == Eye_Left ) ? 0 : kWindowWidth / 2;
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
	uint32_t m_objectId;
	PropertyContainerHandle_t m_propContainer;
	VRInputComponentHandle_t m_proximity = k_ulInvalidInputComponentHandle;
	HmdQuaternion_t m_orientation = { 1, 0, 0, 0 }; // updated by head tracking later
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
		return VRInitError_None;
	}

	void Cleanup() override
	{
		delete m_hmd;
		m_hmd = nullptr;
		VR_CLEANUP_SERVER_DRIVER_CONTEXT();
	}

	const char *const *GetInterfaceVersions() override { return k_InterfaceVersions; }

	void RunFrame() override
	{
		if ( m_hmd )
			m_hmd->RunFrame();
	}

	bool ShouldBlockStandbyMode() override { return false; }
	void EnterStandby() override {}
	void LeaveStandby() override {}

private:
	CVrcBridgeHmd *m_hmd = nullptr;
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
