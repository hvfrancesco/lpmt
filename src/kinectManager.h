#ifndef OF_KINECT_MANAGER
#define OF_KINECT_MANAGER

#include "ofMain.h"
#include "ofxKinect.h"
//#include "ofxOpenCv.h"


//class kinectManager : public ofThread {
class kinectManager {

    public:


        ofxKinect kinect;

        //---------------------------------------------------------
        kinectManager() {
        }

        bool kinectOn;
        int kinectAngle;

        ofImage thDepthImage;
        ofImage grayImage;
        //ofxCvGrayscaleImage grayImage; // the depth image
        //ofxCvGrayscaleImage grayThreshNear; // the near thresholded image
        //ofxCvGrayscaleImage grayThreshFar; // the far thresholded image

        ofTexture grayTexture;


        //---------------------------------------------------------
        //This is called in the setup
        bool setup();

        void update();


        //---------------------------------------------------------
        ofImage getThresholdDepthImage(int nearDepthTh, int farDepthTh);

};

#endif
