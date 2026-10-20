#ifndef OF_KINECT_MANAGER
#define OF_KINECT_MANAGER

#include "ofMain.h"
#include "ofxKinect.h"
#include "ofxOpenCv.h"


class kinectManager {

    public:


        ofxKinect kinect;

        //---------------------------------------------------------
        kinectManager() {
        }

        bool kinectOn;
        int kinectAngle;

        //ofImage thDepthImage;
        //ofImage grayImage;
        ofxCvGrayscaleImage grayImage; // the depth image
        ofxCvGrayscaleImage grayThreshNear; // the near thresholded image
        ofxCvGrayscaleImage grayThreshFar; // the far thresholded image
        ofxCvGrayscaleImage thDepthImage;

        //ofTexture grayTexture;


        //---------------------------------------------------------
        //This is called in the setup
        bool setup();

        void update();


        //---------------------------------------------------------
        ofxCvGrayscaleImage getThresholdDepthImage(int nearDepthTh, int farDepthTh, int blurVal);

};

#endif
