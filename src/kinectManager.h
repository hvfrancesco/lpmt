#ifndef OF_KINECT_MANAGER
#define OF_KINECT_MANAGER

#include "ofMain.h"
//#include "ofThread.h"
#include "ofxKinect.h"
//#include "ofxOpenCv.h"


//class kinectManager : public ofThread {
class kinectManager {

    public:


        ofxKinect kinect;

        //---------------------------------------------------------
        kinectManager() {
        }


        ofImage grayImage;
        //ofxCvGrayscaleImage grayImage;

        ofTexture grayTexture;


        //---------------------------------------------------------
        //This is called in the setup
        void setup();

        void update();


        //---------------------------------------------------------
        void threadedFunction();

};

#endif
