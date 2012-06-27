#ifndef _TEST_APP
#define _TEST_APP

#include "config.h"

#define OF_ADDON_USING_OFXXMLSETTINGS

//#define WINDOW_W 800
//#define WINDOW_H 600

// OSC stuff - listen on port 12345
#define PORT 12345
#define NUM_MSG_STRINGS 20

#include "ofMain.h"

#include "quad.h"
#ifdef WITH_KINECT
#include "kinectManager.h"
#endif
#include "ofxXmlSettings.h"
#include "ofxMostPixelsEver.h"
#include "ofxOsc.h"
#ifdef WITH_TIMELINE
#include "ofxTimeline.h"
#endif
#include "ofxSimpleGuiToo.h"
#ifdef WITH_SYPHON
#include "ofxSyphon.h"
#endif
#ifdef WITH_MIDI
#include "ofxMidi.h"
#endif

//#include <GL/glut.h>

#ifdef WITH_MIDI
class testApp : public ofBaseApp, public ofxMidiListener
#else
class testApp : public ofBaseApp
#endif
{

public:

    int WINDOW_W;
    int WINDOW_H;

    void setup();
    void update();
    void draw();
    void prepare();
    void dostuff();
    void exit();

    void mpeSetup();
    void resync();
    void startProjection();
    void stopProjection();
    void copyQuadSettings(int sourceQuad);


    void keyPressed(int key);
    void keyReleased(int key);
    void mouseMoved(int x, int y );
    void mouseDragged(int x, int y, int button);
    void mousePressed(int x, int y, int button);
    void mouseReleased();
    void windowResized(int w, int h);

    int whichCorner;

    ofTrueTypeFont ttf;

    quad quads[36];
    int layers[36];

    int activeQuad;
    int nOfQuads;
    int borderColor;
    int copyQuadNum;

    bool configOk;
    bool autoStart;

    bool isSetup;
    bool bFullscreen;
    bool bGui;
    bool bTimeline;
    bool snapshotOn;
    bool bStarted;
    bool bSplash;
    bool maskSetup;
    bool gridSetup;
    bool bCameraOk;
    bool bSnapOn;

    ofImage splashImg;
    float splashTime;

    // use of MostPixelsEver
    bool bMpe;
    mpeClientTCP client;
    void mpeFrameEvent(ofxMPEEventArgs& event);
    void mpeMessageEvent(ofxMPEEventArgs& event);
    void mpeResetEvent(ofxMPEEventArgs& event);
    float lastFrameTime;

    // OSC stuff
    ofxOscReceiver	receiver;
    int				current_msg_string;
    string		msg_strings[NUM_MSG_STRINGS];
    float			timers[NUM_MSG_STRINGS];
    void parseOsc();

    // MIDI stuff
    #ifdef WITH_MIDI
    void newMidiMessage(ofxMidiMessage& eventArgs);
    ofxMidiIn midiIn;
	ofxMidiMessage midiMessage;
	#endif

    // Shaders
    ofShader edgeBlendShader;
    ofShader quadMaskShader;
    ofShader chromaShader;

    // gui elements
    bool showGui;

    bool bImageLoad;
    bool bVideoLoad;
    bool bSharedVideoLoad0;
    bool bSharedVideoLoad1;
    bool bSharedVideoLoad2;
    bool bSharedVideoLoad3;
    bool bQuadReset;
    bool bQuadBezierSpherize;
    bool bQuadBezierSpherizeStrong;
    bool bQuadBezierReset;

    void openImageFile();
    void openVideoFile();
    void openSharedVideoFile(int i);
    void openSharedVideoFile(string path, int i);
    void quadDimensionsReset(int q);
    void quadPlacementReset(int q);
    void quadBezierSpherize(int q);
    void quadBezierSpherizeStrong(int q);
    void quadBezierReset(int q);

    float totRotationAngle;
    ofPolyline rotationSector;

    // camera grabber
    //ofVideoGrabber camGrabber;
    ofTexture camTexture;
    ofTexture snapshotTexture;
    int numOfCams;
	vector<ofVideoGrabber> cameras;
	vector<string> cameraIDs;

    vector<ofVideoPlayer> sharedVideos;
    vector<string> sharedVideosFiles;

    int camID;
    int reqCamWidth;
    int reqCamHeight;
    int camWidth;
    int camHeight;

    vector<string> imgFiles;
    vector<string> videoFiles;
    vector<string> slideshowFolders;

    ofxXmlSettings XML;
    void setXml();
    void getXml(string xmlFile);

    // double-click stuff
    unsigned long doubleclickTime;
    unsigned long lastTap;

    void activateQuad(int x, int y);

    ofPoint startDrag;


    // snapshot loading
    ofImage loadImageFromFile();

    string loadSlideshow();
    bool bSlideshowLoad;

    #ifdef WITH_KINECT
    kinectManager kinect;
    bool bKinectOk;
    bool bCloseKinect;
    bool bOpenKinect;
    #endif

    // timeline
    #ifdef WITH_TIMELINE
    ofxTimeline timeline;
    float timelineDurationSeconds;
    float timelinePreviousDuration;
    void timelineSetup(float duration);
    void timelineUpdate();
    void timelineAddQuadPage(int i);
	void timelineTriggerReceived(ofxTLTriggerEventArgs& trigger);
	bool useTimeline;
	#endif

    // syphon
    #ifdef WITH_SYPHON
	ofxSyphonClient syphClient;
    #endif


};

#endif

