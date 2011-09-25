#ifndef _TEST_APP
#define _TEST_APP
#define OF_ADDON_USING_OFXXMLSETTINGS

#define WINDOW_W 800
#define WINDOW_H 600

// OSC stuff - listen on port 12345
#define PORT 12345
#define NUM_MSG_STRINGS 20

#include "ofMain.h"

#include "quad.h"
#include "ofxXmlSettings.h"
#include "ofxMostPixelsEver.h"
#include "ofxOsc.h"

class testApp : public ofSimpleApp
{

public:

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
    bool snapshotOn;
    bool bStarted;
    bool bSplash;

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

    // gui elements
    bool showGui;
    void openImageFile();
    void openVideoFile();
    bool bImageLoad;
    bool bVideoLoad;

    // camera grabber
    ofVideoGrabber camGrabber;
    ofTexture camTexture;
    ofTexture snapshotTexture;

    int camWidth;
    int camHeight;

    vector<string> imgFiles;
    vector<string> videoFiles;
    vector<string> slideshowFolders;

    ofxXmlSettings XML;
    void setXml();
    void getXml();




};

#endif

