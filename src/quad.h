#pragma once

#include "ofMain.h"
#include "ofGraphics.h"
#include "ball.h"

#ifndef True
#define True true
#endif

#ifndef False
#define False false
#endif

class quad
{

public:
    quad()
    {
    }

    ofPoint corners[4];

    ofPoint src[4];
    ofPoint dst[4];
    //lets make a matrix for openGL
    //this will be the matrix that peforms the transformation
    GLfloat matrix[16];
    ofTrueTypeFont ttf;
    ofTrueTypeFont ttf2;
    // img and video stuff
    ofImage img;
    ofImage slide;
    ofVideoPlayer video;
    ofTexture videoTex;

    int videoWidth;
    int videoHeight;
    unsigned char * videoAlphaPixels;
    unsigned char * videoPixels;

    ball balls[80];

    int borderColor;
    ofFloatColor bgColor;
    ofFloatColor secondColor;
    ofFloatColor startColor;
    ofFloatColor endColor;
    ofFloatColor transColor;
    ofFloatColor imgColorize;
    ofFloatColor videoColorize;
    ofFloatColor camColorize;
    ofFloatColor colorGreenscreen;

    int bgAlpha;

    // camera stuff
    ofTexture camTexture;
    ofTexture camAlphaTexture;
    int camWidth;
    int camHeight;
    unsigned char * camPixels;
    unsigned char * camAlphaPixels;

    int layer;

    float camMultX;
    float camMultY;
    float imgMultX;
    float imgMultY;
    float videoMultX;
    float videoMultY;
    float videoSpeed;
    float previousSpeed;
    float slideshowSpeed;
    float transDuration;

    int quadNumber;

    bool initialized;
    bool isSetup;
    bool isOn;
    bool colorBg;
    bool transBg;
    bool transUp;
    bool camBg;
    bool camGreenscreen;
    bool imgBg;
    bool videoBg;
    bool videoSound;
    bool videoLoop;
    bool videoGreenscreen;
    bool slideshowBg;
    bool slideFit;
    bool slideKeepAspect;

    bool videoHFlip;
    bool imgHFlip;
    bool camHFlip;
    bool videoVFlip;
    bool imgVFlip;
    bool camVFlip;

    int videoVolume;
    int thresholdGreenscreen;
    int bgSlideshow;
    int currentSlide;
    int transStep;
    int transCounter;
    int fps;
    int slideFramesDuration;
    int slideTimer;

    vector<string> videos;
    vector<string> slideshows;
    vector<string> slidesnames;
    vector<ofImage> slides;

    string bgImg;
    string bgVideo;
    string loadedImg;
    string loadedVideo;
    string loadedSlideshow;

    // a func for reading a dir content to a vector of strings
    int getdir (string dir, vector<string> &files);


    void setup(float x1, float y1, float x2, float y2, float x3, float y3, float x4, float y4, vector<string> &slideshowFolders);

    void update();

    void draw();

    void gaussian_elimination(float *input, int n);

    void findHomography(ofPoint src[4], ofPoint dst[4], float homography[16]);

    void loadImageFromFile(string imgName, string imgPath);

    void loadVideoFromFile(string videoName, string videoPath);

};

