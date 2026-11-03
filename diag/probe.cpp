// vrcbridge_probe - live OpenVR state dump for diagnosing the virtual rig.
//
// Connects to the RUNNING SteamVR as a background app (does not take over) and
// prints the ground truth of what VRChat also sees: which HMD is active, its
// pose (position / orientation / angular velocity), the controllers (hand role,
// tracked state, pose, type), and the play-space origin. This is how we stop
// guessing — run it while VRChat is in a world and read exactly what's wrong.
//
//   vrcbridge_probe          -> one full dump and exit
//   vrcbridge_probe watch    -> live, ~3 Hz, just the HMD + controllers pose
//                               (move the mouse and watch the numbers change)

#define _USE_MATH_DEFINES
#include "openvr.h"

#include <cstdio>
#include <cmath>
#include <cstring>
#include <string>
#include <thread>
#include <chrono>

using namespace vr;

static void GetPos( const HmdMatrix34_t &m, double &x, double &y, double &z )
{
	x = m.m[0][3];
	y = m.m[1][3];
	z = m.m[2][3];
}

// Approx intrinsic yaw/pitch/roll in degrees from the rotation part.
static void GetEuler( const HmdMatrix34_t &m, double &yaw, double &pitch, double &roll )
{
	yaw = std::atan2( m.m[0][2], m.m[2][2] ) * 180.0 / M_PI;
	double sp = -m.m[1][2];
	if ( sp > 1.0 ) sp = 1.0;
	if ( sp < -1.0 ) sp = -1.0;
	pitch = std::asin( sp ) * 180.0 / M_PI;
	roll = std::atan2( m.m[1][0], m.m[1][1] ) * 180.0 / M_PI;
}

static const char *ClassName( ETrackedDeviceClass c )
{
	switch ( c )
	{
	case TrackedDeviceClass_HMD: return "HMD";
	case TrackedDeviceClass_Controller: return "Controller";
	case TrackedDeviceClass_GenericTracker: return "Tracker";
	case TrackedDeviceClass_TrackingReference: return "Reference";
	default: return "Other";
	}
}

static const char *ResultName( ETrackingResult r )
{
	switch ( r )
	{
	case TrackingResult_Running_OK: return "Running_OK";
	case TrackingResult_Running_OutOfRange: return "Running_OutOfRange";
	case TrackingResult_Calibrating_InProgress: return "Calibrating";
	case TrackingResult_Uninitialized: return "Uninitialized";
	default: return "?";
	}
}

static const char *RoleName( ETrackedControllerRole r )
{
	switch ( r )
	{
	case TrackedControllerRole_LeftHand: return "LeftHand";
	case TrackedControllerRole_RightHand: return "RightHand";
	case TrackedControllerRole_OptOut: return "OptOut";
	default: return "Invalid";
	}
}

static std::string GetStr( IVRSystem *sys, TrackedDeviceIndex_t i, ETrackedDeviceProperty p )
{
	char buf[512] = { 0 };
	ETrackedPropertyError e = TrackedProp_Success;
	sys->GetStringTrackedDeviceProperty( i, p, buf, sizeof( buf ), &e );
	if ( e != TrackedProp_Success )
		return "(n/a)";
	return buf;
}

static void DumpPoseLine( IVRSystem *sys, TrackedDeviceIndex_t i, const TrackedDevicePose_t &pose )
{
	if ( !pose.bPoseIsValid )
	{
		printf( "  pose INVALID (result=%s, connected=%d)\n", ResultName( pose.eTrackingResult ), pose.bDeviceIsConnected );
		return;
	}
	double x, y, z, yaw, pitch, roll;
	GetPos( pose.mDeviceToAbsoluteTracking, x, y, z );
	GetEuler( pose.mDeviceToAbsoluteTracking, yaw, pitch, roll );
	printf( "  pos=(%+.3f %+.3f %+.3f)  yaw=%+6.1f pitch=%+6.1f roll=%+6.1f  result=%s\n",
		x, y, z, yaw, pitch, roll, ResultName( pose.eTrackingResult ) );
	printf( "  angVel=(%+.2f %+.2f %+.2f) rad/s   vel=(%+.2f %+.2f %+.2f) m/s\n",
		pose.vAngularVelocity.v[0], pose.vAngularVelocity.v[1], pose.vAngularVelocity.v[2],
		pose.vVelocity.v[0], pose.vVelocity.v[1], pose.vVelocity.v[2] );
}

static void FullDump( IVRSystem *sys )
{
	uint32_t rw = 0, rh = 0;
	sys->GetRecommendedRenderTargetSize( &rw, &rh );
	printf( "Recommended render target (per eye): %u x %u\n", rw, rh );
	printf( "Seated universe? IsInputAvailable=%d\n", sys->IsInputAvailable() );

	TrackedDevicePose_t poses[k_unMaxTrackedDeviceCount];
	sys->GetDeviceToAbsoluteTrackingPose( TrackingUniverseStanding, 0.0f, poses, k_unMaxTrackedDeviceCount );

	for ( uint32_t i = 0; i < k_unMaxTrackedDeviceCount; i++ )
	{
		ETrackedDeviceClass cls = sys->GetTrackedDeviceClass( i );
		if ( cls == TrackedDeviceClass_Invalid )
			continue;
		printf( "\n[Device %u] class=%s connected=%d\n", i, ClassName( cls ), sys->IsTrackedDeviceConnected( i ) );
		printf( "  Model='%s'  Manuf='%s'  Serial='%s'\n",
			GetStr( sys, i, Prop_ModelNumber_String ).c_str(),
			GetStr( sys, i, Prop_ManufacturerName_String ).c_str(),
			GetStr( sys, i, Prop_SerialNumber_String ).c_str() );
		printf( "  TrackingSystem='%s'  RenderModel='%s'\n",
			GetStr( sys, i, Prop_TrackingSystemName_String ).c_str(),
			GetStr( sys, i, Prop_RenderModelName_String ).c_str() );
		if ( cls == TrackedDeviceClass_Controller )
		{
			ETrackedControllerRole role = sys->GetControllerRoleForTrackedDeviceIndex( i );
			printf( "  Role=%s  ControllerType='%s'  InputProfile='%s'\n",
				RoleName( role ),
				GetStr( sys, i, Prop_ControllerType_String ).c_str(),
				GetStr( sys, i, Prop_InputProfilePath_String ).c_str() );
		}
		DumpPoseLine( sys, i, poses[i] );
	}
}

static void Watch( IVRSystem *sys )
{
	printf( "WATCH mode (~3 Hz). Move the mouse / look around and watch yaw/pitch change. Ctrl+C to stop.\n" );
	for ( ;; )
	{
		TrackedDevicePose_t poses[k_unMaxTrackedDeviceCount];
		sys->GetDeviceToAbsoluteTrackingPose( TrackingUniverseStanding, 0.0f, poses, k_unMaxTrackedDeviceCount );
		for ( uint32_t i = 0; i < k_unMaxTrackedDeviceCount; i++ )
		{
			ETrackedDeviceClass cls = sys->GetTrackedDeviceClass( i );
			if ( cls != TrackedDeviceClass_HMD && cls != TrackedDeviceClass_Controller )
				continue;
			const TrackedDevicePose_t &p = poses[i];
			if ( !p.bPoseIsValid )
			{
				printf( "%-10s INVALID  ", ClassName( cls ) );
				continue;
			}
			double x, y, z, yaw, pitch, roll;
			GetPos( p.mDeviceToAbsoluteTracking, x, y, z );
			GetEuler( p.mDeviceToAbsoluteTracking, yaw, pitch, roll );
			const char *tag = ClassName( cls );
			if ( cls == TrackedDeviceClass_Controller )
				tag = RoleName( sys->GetControllerRoleForTrackedDeviceIndex( i ) );
			printf( "%-9s y=%+.2f yaw=%+6.1f pit=%+6.1f rol=%+6.1f | ", tag, y, yaw, pitch, roll );
		}
		printf( "\n" );
		std::this_thread::sleep_for( std::chrono::milliseconds( 333 ) );
	}
}

int main( int argc, char **argv )
{
	EVRInitError err = VRInitError_None;
	IVRSystem *sys = VR_Init( &err, VRApplication_Background );
	if ( !sys )
	{
		printf( "VR_Init FAILED: %s\n", VR_GetVRInitErrorAsEnglishDescription( err ) );
		printf( "(Is SteamVR running?)\n" );
		return 1;
	}

	printf( "=== vrcbridge_probe : live OpenVR state ===\n" );
	if ( argc > 1 && std::strcmp( argv[1], "watch" ) == 0 )
		Watch( sys );
	else
		FullDump( sys );

	VR_Shutdown();
	return 0;
}
