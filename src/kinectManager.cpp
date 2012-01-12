#include "kinectManager.h"



//---------------------------------------------------------
//This is called in the setup
bool kinectManager::setup()
{

    kinectAngle = 0;

    // enable depth->rgb image calibration
    //kinect.setRegistration(true);

    kinect.init(false,false,true);
    //kinect.init(true); // shows infrared instead of RGB video image
    //kinect.init(false, false); // disable video image (faster fps)
    kinect.open();

    //grayImage.allocate(kinect.width, kinect.height, OF_IMAGE_GRAYSCALE);
    grayImage.allocate(kinect.width, kinect.height);
    grayThreshNear.allocate(kinect.width, kinect.height);
    grayThreshFar.allocate(kinect.width, kinect.height);
    //thDepthImage.allocate(kinect.width, kinect.height, OF_IMAGE_GRAYSCALE);
    thDepthImage.allocate(kinect.width, kinect.height);

    kinectOn = kinect.isConnected();

    return kinectOn;

}

//---------------------------------------------------------
void kinectManager::update()
{

    kinect.update();
    if(kinect.isFrameNew())
    {

        // load grayscale depth image from the kinect source
        grayImage.setFromPixels(kinect.getDepthPixels(), kinect.width, kinect.height);
        //grayImage.setFromPixels(kinect.getDepthPixels(), kinect.width, kinect.height, OF_IMAGE_GRAYSCALE, false);
        if (kinectAngle != kinect.getCurrentCameraTiltAngle() && kinectAngle != kinect.getTargetCameraTiltAngle())
        {
            kinect.setCameraTiltAngle(kinectAngle);
        }

    }

}

//---------------------------------------------------------
ofxCvGrayscaleImage kinectManager::getThresholdDepthImage(int nearDepthTh, int farDepthTh, int blurVal)
{

    int nearThreshold = nearDepthTh;
    int farThreshold = farDepthTh;

    grayThreshNear = grayImage;
    grayThreshFar = grayImage;
    thDepthImage = grayImage;
    grayThreshNear.threshold(nearThreshold, true);
    grayThreshFar.threshold(farThreshold);
    cvAnd(grayThreshNear.getCvImage(), grayThreshFar.getCvImage(), thDepthImage.getCvImage(), NULL);

    thDepthImage.flagImageChanged();
    //thDepthImage.blurGaussian(blurVal);
    thDepthImage.blur(blurVal);

    return thDepthImage;

}


