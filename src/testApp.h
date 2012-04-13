#ifndef _TEST_APP
#define _TEST_APP
#define OF_ADDON_USING_OFXXMLSETTINGS

//#define WINDOW_W 800
//#define WINDOW_H 600

// OSC stuff - listen on port 12345
#define PORT 12345
#define NUM_MSG_STRINGS 20

#include "ofMain.h"

#include "quad.h"
#include "kinectManager.h"
#include "ofxXmlSettings.h"
#include "ofxMostPixelsEver.h"
#include "ofxOsc.h"
#include "ofxTimeline.h"
#include "ofxSimpleGuiToo.h"

class testApp : public ofBaseApp
{

public:

    int WINDOW_W;
    int WINDOW_H;

    void setup();
    void update();
    void draw();
    void prepare();
    void dostuff();

    void mpeSetup();
    void resync();
    void startProjection();
    void stopProjection();


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

    // Shaders
    ofShader edgeBlendShader;
    ofShader quadMaskShader;
    ofShader chromaShader;

    // gui elements
    bool showGui;

    bool bImageLoad;
    bool bVideoLoad;
    bool bQuadReset;
    bool bQuadBezierSpherize;
    bool bQuadBezierSpherizeStrong;
    bool bQuadBezierReset;

    void openImageFile();
    void openVideoFile();
    void quadDimensionsReset(int q);
    void quadPlacementReset(int q);
    void quadBezierSpherize(int q);
    void quadBezierSpherizeStrong(int q);
    void quadBezierReset(int q);

    // camera grabber
    //ofVideoGrabber camGrabber;
    ofTexture camTexture;
    ofTexture snapshotTexture;
    int numOfCams;
	vector<ofVideoGrabber> cameras;
	vector<string> cameraIDs;

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
    void getXml();

    // double-click stuff
    unsigned long doubleclickTime;
    unsigned long lastTap;

    void activateQuad(int x, int y);

    ofPoint startDrag;


    // snapshot loading
    ofImage loadImageFromFile();

    string loadSlideshow();
    bool bSlideshowLoad;

    kinectManager kinect;
    bool bKinectOk;
    bool bCloseKinect;
    bool bOpenKinect;

    // timeline
    ofxTimeline timeline;
    float timelineDurationSeconds;
    float timelinePreviousDuration;
    void timelineSetup(float duration);
    void timelineUpdate();
    void timelineAddQuadPage(int i);
	void timelineTriggerReceived(ofxTLTriggerEventArgs& trigger);
	bool useTimeline;



};

#endif

