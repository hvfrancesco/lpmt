#pragma once

#include "ofMain.h"
#include "ofGraphics.h"
#include "ball.h"

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

    ball balls[80];

    int borderColor;
    ofColor bgColor;
    ofColor secondColor;
    ofColor startColor;
    ofColor endColor;
    ofColor transColor;
    ofColor imgColorize;
    ofColor videoColorize;
    ofColor camColorize;

    int bgAlpha;
    // camera stuff
    ofTexture camTexture;
    int camWidth;
    int camHeight;
    int videoWidth;
    int videoHeight;
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
    float slideTimer;
    float transDuration;

    int quadNumber;

    bool initialized;
    bool isSetup;
    bool isOn;
    bool colorBg;
    bool transBg;
    bool transUp;
    bool camBg;
    bool imgBg;
    bool videoBg;
    bool videoSound;
    bool slideshowBg;
    bool slideFit;
    bool slideKeepAspect;

    int bgImg;
    int bgVideo;
    int videoVolume;
    int bgSlideshow;
    int currentSlide;
    int transStep;
    int transCounter;
    int fps;

    vector<string> images;
    vector<string> videos;
    vector<string> slideshows;
    vector<string> slidesnames;
    vector<ofImage> slides;

    string loadedImg;
    string loadedVideo;
    string loadedSlideshow;

    // a func for reading a dir content to a vector of strings
    int getdir (string dir, vector<string> &files);


    void setup(float x1, float y1, float x2, float y2, float x3, float y3, float x4, float y4, vector<string> &imgFiles, vector<string> &videoFiles, vector<string> &slideshowFolders);

    void update();

    void draw();

    void gaussian_elimination(float *input, int n);

    void findHomography(ofPoint src[4], ofPoint dst[4], float homography[16]);

};

