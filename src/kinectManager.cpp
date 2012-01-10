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
    //grayImage.allocate(kinect.width, kinect.height);
    thDepthImage.allocate(kinect.width, kinect.height, OF_IMAGE_GRAYSCALE);

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
        //grayImage.setFromPixels(kinect.getDepthPixels(), kinect.width, kinect.height);
        grayImage.setFromPixels(kinect.getDepthPixels(), kinect.width, kinect.height, OF_IMAGE_GRAYSCALE, false);
        if (kinectAngle != kinect.getCurrentCameraTiltAngle() && kinectAngle != kinect.getTargetCameraTiltAngle())
        {
            kinect.setCameraTiltAngle(kinectAngle);
        }

    }

}

//---------------------------------------------------------
ofImage kinectManager::getThresholdDepthImage(int nearDepthTh, int farDepthTh)
{

    int nearThreshold = nearDepthTh;
    int farThreshold = farDepthTh;

    unsigned char * depthPix = grayImage.getPixels();
    unsigned char * thPix = thDepthImage.getPixels();

    int numPixels = grayImage.getWidth() * grayImage.getHeight();

    for(int i = 0; i < numPixels; i++)
    {
        if(depthPix[i] < nearThreshold && depthPix[i] > farThreshold)
        {
            thPix[i] = 255;
        }
        else
        {
            thPix[i] = 0;
        }
    }

    thDepthImage.update();
    return thDepthImage;

}
