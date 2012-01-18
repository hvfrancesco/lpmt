#include "quad.h"
#include "homography.h"

#include <sys/types.h>
#include <dirent.h>
#include <errno.h>
#include <vector>
#include <string>

//GLfloat bezierCtrlPoints[4][4][3];
//float bezierCtrlPoints[4][4][3];

// a func for reading a dir content to a vector of strings
int quad::getdir (string dir, vector<string> &files)
{
    DIR *dp;
    struct dirent *dirp;
    if((dp  = opendir(dir.c_str())) == NULL)
    {
        cout << "Error(" << errno << ") opening " << dir << endl;
        return errno;
    }

    while ((dirp = readdir(dp)) != NULL)
    {
        files.push_back(string(dirp->d_name));
    }
    closedir(dp);
    return 0;
}


void quad::setup(float x1, float y1, float x2, float y2, float x3, float y3, float x4, float y4, vector<string> &slideshowFolders, ofShader &edgeBlendShader, ofShader &quadMaskShader, ofVideoGrabber &camGrabber, kinectManager &kinect)
{

    shaderBlend = &edgeBlendShader;
    maskShader = &quadMaskShader;
    camera = &camGrabber;
    quadKinect = &kinect;

    //loads load in some truetype fonts
    //ttf.loadFont("type/frabk.ttf", 11);
    ttf.loadFont("type/OpenSans-Regular.ttf", 11);


    bgImg = string("");
    loadedImg = string("");
    loadedVideo = string("");
    loadedSlideshow = string("");

    quadNumber = 0;
    //this is just for our gui / mouse handles
    //this will end up being the destination quad we are warping too

    corners[0].x = x1;
    corners[0].y = y1;

    corners[1].x = x2;
    corners[1].y = y2;

    corners[2].x = x3;
    corners[2].y = y3;

    corners[3].x = x4;
    corners[3].y = y4;

    center = (corners[0]+corners[1]+corners[2]+corners[3])/4;

    //videos = videoFiles;
    slideshows = slideshowFolders;

    borderColor = 0x666666;

    // sets default variables
    initialized = True;
    isActive = False;
    isSetup = True;
    isOn = True;
    isMaskSetup = False;
    colorBg = False;
    transBg = False;
    transUp = True;
    camBg = False;
    camGreenscreen = False;
    imgBg = False;
    videoBg = False;
    videoLoop = True;
    videoGreenscreen = False;
    slideshowBg = False;
    slideFit = False;
    slideKeepAspect = True;

    videoHFlip = False;
    imgHFlip = False;
    camHFlip = False;
    videoVFlip = False;
    imgVFlip = False;
    camVFlip = False;

    camWidth = 640;
    camHeight = 480;
    camMultX = 1;
    camMultY = 1;
    camTexture.allocate(camWidth,camHeight, GL_RGB);
    camAlphaTexture.allocate(camWidth, camHeight, GL_RGBA);
    camPixels = new unsigned char [camWidth*camHeight*3];
    camAlphaPixels = new unsigned char [camWidth*camHeight*4];

    imgMultX = 1.0;
    imgMultY = 1.0;

    videoMultX = 1.0;
    videoMultY = 1.0;
    videoSpeed = 1.0;
    previousSpeed = 1.0;
    videoVolume = 0;

    currentSlide = 0;
    slideshowSpeed = 1.0;
    slideTimer = ofGetElapsedTimef();

    transDuration = 1.0;
    fps = ofGetFrameRate();
    transCounter = 0;

    bgColor.r = 0;
    bgColor.g = 0;
    bgColor.b = 0;
    bgColor.a = 0;

    secondColor.r = 0;
    secondColor.g = 0;
    secondColor.b = 0;
    secondColor.a = 0;

    startColor.r = 0;
    startColor.g = 0;
    startColor.b = 0;
    startColor.a = 0;

    endColor.r = 0;
    endColor.g = 0;
    endColor.b = 0;
    endColor.a = 0;

    imgColorize.r = 1;
    imgColorize.g = 1;
    imgColorize.b = 1;
    imgColorize.a = 1;

    videoColorize.r = 1;
    videoColorize.g = 1;
    videoColorize.b = 1;
    videoColorize.a = 1;

    camColorize.r = 1;
    camColorize.g = 1;
    camColorize.b = 1;
    camColorize.a = 1;

    colorGreenscreen.r = 0;
    colorGreenscreen.g = 0;
    colorGreenscreen.b = 0;
    colorGreenscreen.a = 0;

    thresholdGreenscreen = 10;

    kinectBg = false;
    kinectImg = false;
    kinectMask = false;
    kinectMultX = 1.0;
    kinectMultY = 1.0;
    kinectColorize.r = 1.0;
    kinectColorize.g = 1.0;
    kinectColorize.b = 1.0;
    kinectColorize.a = 1.0;
    nearDepthTh = 255;
    farDepthTh = 0;
    kinectBlur = 3;
    kinectContourMin = 0.01;
    kinectContourMax = 1.0;
    kinectContourSimplify = 0.0;
    kinectContourSmooth = 10;
    getKinectContours = false;
    getKinectGrayImage = false;
    kinectContourCurved = false;

    edgeBlendBool = False;
    edgeBlendExponent = 1.0;
    edgeBlendGamma = 1.8;
    edgeBlendLuminance = 0.0;
    edgeBlendAmountSin = 0.3;
    edgeBlendAmountDx = 0.3;
    edgeBlendAmountTop = 0.0;
    edgeBlendAmountBottom = 0.0;

    settings.internalformat = GL_RGBA;
    settings.numSamples = 0;
    settings.useDepth = false;
    settings.useStencil = false;
    settings.width = ofGetWidth();
    settings.height = ofGetHeight();

    quadDispX = 0;
    quadDispY = 0;
    quadW = ofGetWidth();
    quadH = ofGetHeight();

    bBlendModes = False;
    blendMode = 0;

    bHighlightCorner = False;
    highlightedCorner = -1;

    maskPoints = vector<ofPoint>();
    bMask = False;
    maskInvert = False;

    bHighlightMaskPoint = False;
    highlightedMaskPoint = -1;

    bDeform = False;

    //This sets up my Bezier Surface
    bBezier = False;
    isBezierSetup = False;
    highlightedCtrlPointRow = -1;
    highlightedCtrlPointCol = -1;

    // prepare bezier surface evaluator with control points
    bezierSurfaceSetup();

    // prepare grid surface evaluator
    bGrid = False;
    gridColumns = 8;
    gridRows = 6;
    gridSurfaceSetup();

}


void quad::update()
{
    if (isOn)
    {

        //recalculates center of quad
        center = (corners[0]+corners[1]+corners[2]+corners[3])/4;

        // solid colors ---------------------------------------------------------------
        // calculates transition between two solid colors
        if (colorBg && transBg)
        {
            if (transUp)
            {
                startColor = bgColor;
                endColor = secondColor;
            }
            else
            {
                startColor = secondColor;
                endColor = bgColor;
            }
            // using fps detected at setup is suboptimal
            // but updating it at each cycle triggers a flickering effect
            // needs more work (e.g. introducing a roundup of actual framerate)
            // now we update fps value just if it differs more than 50fps from actual rate
            //transStep = (transDuration * ofGetFrameRate());
            //if (abs(fps-ofGetFrameRate()) > 50) {fps = ofGetFrameRate();}
            transStep = (transDuration * fps);
            transColor.r = startColor.r + (((endColor.r - startColor.r)/transStep)*transCounter);
            transColor.g = startColor.g + (((endColor.g - startColor.g)/transStep)*transCounter);
            transColor.b = startColor.b + (((endColor.b - startColor.b)/transStep)*transCounter);
            transColor.a = startColor.a + (((endColor.a - startColor.a)/transStep)*transCounter);
            transCounter += 1;
            if (transCounter >= transStep)
            {
                transCounter = 0;
                transUp = !transUp;
            }

        }


        // live camera --------------------------------------------------------------
        if (camBg && camera->width > 0)
        {
            camPixels = camera->getPixels();
            if (camGreenscreen)
            {
                // checking for greenscreen color match
                for (int i = 0; i < camWidth*camHeight; i++)
                {
                    int deltaR = abs(camPixels[i*3+0] - colorGreenscreen.r*255);
                    int deltaG = abs(camPixels[i*3+1] - colorGreenscreen.g*255);
                    int deltaB = abs(camPixels[i*3+2] - colorGreenscreen.b*255);
                    if (deltaR <= thresholdGreenscreen && deltaG <= thresholdGreenscreen && deltaB <= thresholdGreenscreen)
                    {
                        camAlphaPixels[i*4+3] = 0;
                    }
                    else
                    {
                        camAlphaPixels[i*4+3] = 255;
                    }
                    // RGB data is copied untouched
                    camAlphaPixels[i*4+0] = camPixels[i*3+0];
                    camAlphaPixels[i*4+1] = camPixels[i*3+1];
                    camAlphaPixels[i*4+2] = camPixels[i*3+2];

                }
                // loads data into texture with alpha
                camAlphaTexture.loadData(camAlphaPixels,camWidth,camHeight,GL_RGBA);
            }
            else
            {
                // loads camera pixels into this quad camera-texture wirh no alpha
                camTexture.loadData(camPixels, camWidth, camHeight, GL_RGB);
            }
        }

        // video --------------------------------------------------------------------
        // loads video
        if (videoBg)
        {
            video.setVolume(videoVolume);
            // check for looping config parameter of video and sets loopstate - OF_LOOP_NORMAL = cycles / OF_LOOP_NONE = stops at the end
            if (videoLoop)
            {
                video.setLoopState(OF_LOOP_NORMAL);
            }
            else
            {
                video.setLoopState(OF_LOOP_NONE);
            }

            if (video.isLoaded())
            {
                video.idleMovie();
            }

            // video greenscreen stuff
            if (videoGreenscreen)
            {
                // gets video frame pixels array
                videoPixels = video.getPixels();
                // checking for greenscreen color match
                for (int i = 0; i < videoWidth*videoHeight; i++)
                {
                    int deltaR = abs(videoPixels[i*3+0] - colorGreenscreen.r*255);
                    int deltaG = abs(videoPixels[i*3+1] - colorGreenscreen.g*255);
                    int deltaB = abs(videoPixels[i*3+2] - colorGreenscreen.b*255);
                    if (deltaR <= thresholdGreenscreen && deltaG <= thresholdGreenscreen && deltaB <= thresholdGreenscreen)
                    {
                        videoAlphaPixels[i*4+3] = 0;
                    }
                    else
                    {
                        videoAlphaPixels[i*4+3] = 255;
                    }
                    // RGB data is copied untouched
                    videoAlphaPixels[i*4+0] = videoPixels[i*3+0];
                    videoAlphaPixels[i*4+1] = videoPixels[i*3+1];
                    videoAlphaPixels[i*4+2] = videoPixels[i*3+2];
                }
                videoTex.loadData(videoAlphaPixels,videoWidth,videoHeight,GL_RGBA);
            }

            // changevideo speed
            if (previousSpeed != videoSpeed)
            {
                video.setSpeed(videoSpeed);
                previousSpeed = videoSpeed;
            }
        }

        // slideshow -----------------------------------------------------------------
        if (slideshowBg)
        {
            // put it to off while loading images
            slideshowBg = False;
            string slideshowName = slideshows[bgSlideshow];
            // if a different slideshow has been chosen in gui we do load its images
            if (slideshowName != loadedSlideshow)
            {
                // we exclude "." and ".." directories if present
                if (slideshowName != "." && slideshowName != "..")
                {
                    // we scan the img dir for images
                    string slidesDir = string("./data/slideshow/");
                    slidesDir += slideshowName;
                    // make two arrays, one for imgs names and one for images
                    slidesnames = vector<string>();
                    slides = vector<ofImage>();
                    // read all content of show folder
                    getdir(slidesDir,slidesnames);
                    // for each name found loads the image and populates the imgs array (excluding "." an "..")
                    for (unsigned int i = 0; i < slidesnames.size(); i++)
                    {
                        if (slidesnames[i] != "." && slidesnames[i] != "..")
                        {
                            slide.loadImage("slideshow/"+slideshowName+"/"+slidesnames[i]);
                            slides.push_back(slide);
                        }
                    }
                    loadedSlideshow = slideshowName;
                    currentSlide = 0;
                    slideTimer = 0;
                }
            }
            // turn it on again for drawing
            // update fps
            //if (abs(fps-ofGetFrameRate()) > 50) {fps = ofGetFrameRate();}
            slideFramesDuration = (slideshowSpeed * fps);
            slideshowBg = True;
        }

        // -------------------------
        // finds kinect blobs with OpenCV
        if (kinectBg)
        {
            kinectThreshImage.clear();
            kinectContourImage.clear();
            kinectThreshImage.allocate(quadKinect->kinect.width, quadKinect->kinect.height);
            kinectContourImage.allocate(quadKinect->kinect.width, quadKinect->kinect.height);
            kinectThreshImage = quadKinect->getThresholdDepthImage(nearDepthTh, farDepthTh, kinectBlur);
            kinectContourImage = kinectThreshImage;
            kinectContourFinder.findContours(kinectContourImage, (quadKinect->kinect.width*quadKinect->kinect.height)*kinectContourMin, (quadKinect->kinect.width*quadKinect->kinect.height)*kinectContourMax, 20, true);
            // clear kinect path if any
            kinectPath.clear();
            kinectPath.setFilled(true);
            for( int i=0; i<(int)kinectContourFinder.blobs.size(); i++ )
            {
                ofPolyline poly(kinectContourFinder.blobs[i].pts);
                poly.close();
                poly.simplify(kinectContourSimplify);
                ofPolyline polySmoothed = poly.getSmoothed(kinectContourSmooth);
                //polySmoothed.close();
                vector<ofPoint> points = polySmoothed.getVertices();

                for( int j=0; j<points.size(); j++ )
                {
                    if (kinectContourCurved)
                    {
                        kinectPath.curveTo(points[j]);
                    }
                    else
                    {
                        kinectPath.lineTo(points[j]);
                    }
                }
                kinectPath.close();
            }
        }

        //we set matrix to the default - 0 translation
        //and 1.0 scale for x y z and w
        for(int i = 0; i < 16; i++)
        {
            if(i % 5 != 0) matrix[i] = 0.0;
            else matrix[i] = 1.0;
        }

        //we set the warp coordinates
        //source coordinates as the dimensions of our window
        src[0].x = 0;
        src[0].y = 0;
        src[1].x = ofGetWidth();
        src[1].y = 0;
        src[2].x = ofGetWidth();
        src[2].y = ofGetHeight();
        src[3].x = 0;
        src[3].y = ofGetHeight();

        //corners are in 0.0 - 1.0 range
        //so we scale up so that they are at the window's scale
        for(int i = 0; i < 4; i++)
        {
            dst[i].x = corners[i].x  * (float)ofGetWidth();
            dst[i].y = corners[i].y * (float) ofGetHeight();
        }

    }
}


// DRAW -----------------------------------------------------------------
void quad::draw()
{
    if (isOn)
    {

        // recalculates bezier surface
        if(bBezier)
        {
            bezierSurfaceUpdate();
        }
        if(bGrid)
        {
            gridSurfaceUpdate();
        }


        quadFbo.begin();
        ofClear(0.0,0.0,0.0,0.0);
        ofEnableAlphaBlending();
        // -- NOW LETS DRAW!!!!!!  -----

        // if a solid color content or color transition is set it draws it
        // solid colors ----------------------------------------------------------------
        if (colorBg)
        {
            ofFill();
            // if we have two colors it draws with calculated transition color
            if (transBg)
            {
                ofSetColor(transColor.r * 255, transColor.g * 255, transColor.b * 255, transColor.a * 255);
            }
            // this in case of only one color set
            else
            {
                ofSetColor(bgColor.r * 255, bgColor.g * 255, bgColor.b * 255, bgColor.a * 255);
            }
            ofRect(0, 0, ofGetWidth(), ofGetHeight());
            ofNoFill();
        }

        // video ----------------------------------------------------------------------
        //if a video content is chosen it draws it
        if (videoBg)
        {
            if (videoHFlip || videoVFlip)
            {
                glPushMatrix();
                if(videoHFlip && !videoVFlip)
                {
                    ofTranslate(videoWidth*videoMultX,0);
                    glScalef(-1,1,1);
                }
                else if(videoVFlip && !videoHFlip)
                {
                    ofTranslate(0,videoHeight*videoMultY);
                    glScalef(1,-1,1);
                }
                else
                {
                    ofTranslate(videoWidth*videoMultX,videoHeight*videoMultY);
                    glScalef(-1,-1,1);
                }
            }
            ofSetColor(videoColorize.r * 255, videoColorize.g * 255, videoColorize.b * 255, videoColorize.a * 255);
            if (!videoLoop)
            {
                // in no-looping mode it stops drawing video frame when video reaches the end
                // using 'getIsMovieDone()' because there are problems with getting head position under GStream
                if (!video.getIsMovieDone())
                {
                    if (videoGreenscreen)
                    {
                        videoTex.draw(0,0,videoWidth*videoMultX, videoHeight*videoMultY);
                    }
                    else
                    {
                        video.draw(0,0,videoWidth*videoMultX, videoHeight*videoMultY);
                    }
                }
            }
            else
            {
                if (videoGreenscreen)
                {
                    videoTex.draw(0,0,videoWidth*videoMultX, videoHeight*videoMultY);
                }
                else
                {
                    video.draw(0,0,videoWidth*videoMultX, videoHeight*videoMultY);
                }
            }
            if (videoHFlip || videoVFlip)
            {
                glPopMatrix();
            }
        }


        // camera ------------------------------------------------------------------------------
        // camera stuff
        if (camBg && camera->width > 0)
        {
            if (camHFlip || camVFlip)
            {
                glPushMatrix();
                if(camHFlip && !camVFlip)
                {
                    ofTranslate(camWidth*camMultX,0);
                    glScalef(-1,1,1);
                }
                else if(camVFlip && !camHFlip)
                {
                    ofTranslate(0,camHeight*camMultY);
                    glScalef(1,-1,1);
                }
                else
                {
                    ofTranslate(camWidth*camMultX,camHeight*camMultY);
                    glScalef(-1,-1,1);
                }
            }
            ofSetColor(camColorize.r * 255, camColorize.g * 255, camColorize.b * 255, camColorize.a * 255);
            if (camGreenscreen)
            {
                camAlphaTexture.draw(0,0,camWidth*camMultX,camHeight*camMultY);
            }
            else
            {
                camTexture.draw(0,0,camWidth*camMultX,camHeight*camMultY);
            }
            if (camHFlip || camVFlip)
            {
                glPopMatrix();
            }
        }

        // draws slideshows
        if (slideshowBg)
        {
            if (slides.size() > 0)
            {
                // if we reached the end of slides vector, it loops back to first slide
                if (currentSlide >= slides.size())
                {
                    currentSlide = 0;
                }
                slide = slides[currentSlide];
                // color is set according to still img colorization combo
                ofSetColor(imgColorize.r * 255, imgColorize.g * 255, imgColorize.b * 255, imgColorize.a * 255);
                // default is drawing image with its size unchanged, so we set mult factors = 1.0
                float multX = 1.0;
                float multY = 1.0;
                if (slideFit)
                {
                    float fitX = ofGetWidth()/slide.getWidth();
                    float fitY = ofGetHeight()/slide.getHeight();
                    if (slideKeepAspect)
                    {
                        // we calculate the factor for fitting the image in quad respecting img aspect ratio
                        if (fitX >= fitY)
                        {
                            multX = fitY;
                            multY = fitY;
                        }
                        else
                        {
                            multX = fitX;
                            multY = fitX;
                        }
                    }
                    // this is for stretching image to whole quad size
                    else
                    {
                        multX = fitX;
                        multY = fitY;
                    }
                }
                // at last we draw the image with appropriate size multiplier
                slide.draw(0,0,slide.getWidth()*multX, slide.getHeight()*multY);

                // if slide showing time has elapsed it switches to next slide
                if (slideTimer > slideFramesDuration )
                {
                    currentSlide += 1;
                    slideTimer = 0;
                }
                slideTimer += 1;
            }
        }


        //if an image content is chosen it draws it (maybe use it as mask as well?)
        if (imgBg)
        {
            if (imgHFlip || imgVFlip)
            {
                glPushMatrix();
                if(imgHFlip && !imgVFlip)
                {
                    ofTranslate(img.getWidth()*imgMultX,0);
                    glScalef(-1,1,1);
                }
                else if(imgVFlip && !imgHFlip)
                {
                    ofTranslate(0,img.getHeight()*imgMultY);
                    glScalef(1,-1,1);
                }
                else
                {
                    ofTranslate(img.getWidth()*imgMultX,img.getHeight()*imgMultY);
                    glScalef(-1,-1,1);
                }
            }
            ofSetColor(imgColorize.r * 255, imgColorize.g * 255, imgColorize.b * 255, imgColorize.a * 255);
            img.draw(0,0,img.getWidth()*imgMultX, img.getHeight()*imgMultY);
            if (imgHFlip || imgVFlip)
            {
                glPopMatrix();
            }
        }

        // kinect stuff
        if (kinectBg && kinectImg)
        {
            ofSetColor(kinectColorize.r * 255, kinectColorize.g * 255, kinectColorize.b * 255, kinectColorize.a * 255);
            //quadKinect->grayImage.draw(0,0,quadKinect->grayImage.getWidth()*kinectMultX,quadKinect->grayImage.getHeight()*kinectMultY);
            if (getKinectContours)
            {
                ofPushStyle();
                glPushMatrix();
                glScalef( kinectMultX, kinectMultY, 0.0 );
                // ----------- draw the kinect path made of detected blobs
                ofColor pathColor(kinectColorize.r * 255, kinectColorize.g * 255, kinectColorize.b * 255, kinectColorize.a * 255);
                kinectPath.setFillColor(pathColor);
                kinectPath.draw();
                glPopMatrix();
                ofPopStyle();
            }
            else if (getKinectGrayImage)
            {
                quadKinect->grayImage.draw(0,0,quadKinect->grayImage.getWidth()*kinectMultX,quadKinect->grayImage.getHeight()*kinectMultY);
            }
            else
            {
                kinectThreshImage.draw(0,0,quadKinect->grayImage.getWidth()*kinectMultX,quadKinect->grayImage.getHeight()*kinectMultY);
            }
        }

        ofDisableAlphaBlending();
        quadFbo.end();

        //mask on mask FBO
        maskFbo.begin();
        ofClear(0.0,0.0,0.0,0.0);
        ofEnableAlphaBlending();
        ofFill();
        ofEnableSmoothing();
        if(maskPoints.size()>0)
        {
            ofSetColor(255,255,255);
            ofBeginShape();
            for(unsigned int i = 0; i < maskPoints.size(); i++)
            {
                ofVertex(maskPoints[i]);
            }
            ofEndShape(true);
        }
        if(kinectBg && kinectMask)
        {
            ofSetColor(255,255,255);
            if (getKinectContours)
            {
                ofPushStyle();
                glPushMatrix();
                glScalef( kinectMultX, kinectMultY, 0.0 );
                // ----------- draw the kinect path made of detected blobs
                ofColor pathColor(255, 255, 255, 255);
                kinectPath.setFillColor(pathColor);
                kinectPath.draw();
                glPopMatrix();
                ofPopStyle();
            }
            else
            {
                kinectThreshImage.draw(0,0,quadKinect->grayImage.getWidth()*kinectMultX,quadKinect->grayImage.getHeight()*kinectMultY);
            }
        }
        ofDisableSmoothing();
        ofNoFill();
        ofDisableAlphaBlending();
        maskFbo.end();

        // save actual GL coordinates
        ofPushMatrix();
        // find transformation matrix
        findHomography(src, dst, matrix);
        //finally lets multiply our matrix
        //wooooo hoooo!
        glMultMatrixf(matrix);


        if(edgeBlendBool)
        {
            if(quadFbo.getWidth()>0)
            {
                ofEnableAlphaBlending();
                shaderBlend->begin();
                shaderBlend->setUniformTexture ("tex", quadFbo.getTextureReference(), 0);
                shaderBlend->setUniform1f("exponent", edgeBlendExponent);
                shaderBlend->setUniform1f("userGamma", edgeBlendGamma);
                shaderBlend->setUniform3f("userLuminance", edgeBlendLuminance, edgeBlendLuminance, edgeBlendLuminance);
                shaderBlend->setUniform4f("amount", edgeBlendAmountSin, edgeBlendAmountTop, edgeBlendAmountDx, edgeBlendAmountBottom);
                shaderBlend->setUniform1i("w", ofGetWidth());
                shaderBlend->setUniform1i("h", ofGetHeight());
                //set ofColor to white
                ofSetColor(255,255,255);
                //Blend modes stuff (with shaders would be better, but it scales bad on older GPUs)
                if(bBlendModes)
                {
                    glEnable(GL_BLEND);
                    if(blendMode == 0) glBlendFunc(GL_ONE, GL_ONE_MINUS_SRC_COLOR); //screen
                    else if(blendMode == 1) glBlendFunc(GL_ONE, GL_ONE); //add
                    else if(blendMode == 2) glBlendFunc(GL_DST_COLOR, GL_SRC_COLOR); // subtract
                    else if(blendMode == 3) glBlendFunc(GL_DST_COLOR, GL_ONE_MINUS_SRC_ALPHA); // multiply
                }
                if(!bDeform)
                {
                    quadFbo.draw(0+quadDispX,0+quadDispY,quadW,quadH);
                    shaderBlend->end();
                }
                else
                {
                    targetFbo.begin();
                    ofClear(0.0,0.0,0.0,0.0);
                    quadFbo.draw(0+quadDispX,0+quadDispY,quadW,quadH);
                    shaderBlend->end();
                    targetFbo.end();

                    targetFbo.getTextureReference().bind();

                    glMatrixMode(GL_TEXTURE);
                    glPushMatrix();//to scale the texture
                    glLoadIdentity();

                    ofTextureData texData = targetFbo.getTextureReference().getTextureData();
                    if(texData.textureTarget == GL_TEXTURE_RECTANGLE_ARB)
                    {
                        glScalef(targetFbo.getTextureReference().getWidth(), targetFbo.getTextureReference().getHeight(), 1.0f);
                    }
                    else
                    {
                        glScalef(targetFbo.getTextureReference().getWidth() / texData.tex_w, targetFbo.getTextureReference().getHeight() / texData.tex_h, 1.0f);
                    }
                    glMatrixMode(GL_MODELVIEW);

                    glEnable(GL_MAP2_VERTEX_3);
                    glEnable(GL_AUTO_NORMAL);
                    // this tries to prevent the double alpha problem
                    glEnable(GL_BLEND);
                    glBlendFunc(GL_ONE, GL_ONE_MINUS_SRC_ALPHA);
                    //draw the bezier shape
                    glEvalMesh2(GL_FILL, 0, 20, 0, 20);
                    glDisable(GL_BLEND);
                    glDisable(GL_MAP2_VERTEX_3);
                    glDisable(GL_AUTO_NORMAL);

                    targetFbo.getTextureReference().unbind();
                    glMatrixMode(GL_TEXTURE);
                    glPopMatrix();// texture scale pop matrix
                    glMatrixMode(GL_MODELVIEW);
                }

                if(bBlendModes)
                {
                    glDisable(GL_BLEND);
                }
                ofDisableAlphaBlending();

            }
        }

        else
        {

            if(bMask)
            {
                if(quadFbo.getWidth()>0)
                {
                    ofEnableAlphaBlending();
                    if (maskInvert)
                    {
                        maskMode = 1;
                    }
                    else
                    {
                        maskMode = 0;
                    }
                    maskShader->begin();
                    maskShader->setUniformTexture ("tex", quadFbo.getTextureReference(), 0);
                    maskShader->setUniformTexture ("mask", maskFbo.getTextureReference(), 1);
                    maskShader->setUniform1i ("mode", maskMode);
                    //set ofColor to white
                    ofSetColor(255,255,255);
                    //Blend modes stuff (with shaders would be better, but it scales bad on older GPUs)
                    if(bBlendModes)
                    {
                        glEnable(GL_BLEND);
                        if(blendMode == 0) glBlendFunc(GL_ONE, GL_ONE_MINUS_SRC_COLOR); //screen
                        else if(blendMode == 1) glBlendFunc(GL_ONE, GL_ONE); //add
                        else if(blendMode == 2) glBlendFunc(GL_DST_COLOR, GL_SRC_COLOR); // subtract
                        else if(blendMode == 3) glBlendFunc(GL_DST_COLOR, GL_ONE_MINUS_SRC_ALPHA); // multiply
                    }
                    if(!bDeform)
                    {
                        quadFbo.draw(0+quadDispX,0+quadDispY,quadW,quadH);
                        maskShader->end();
                    }
                    else
                    {
                        targetFbo.begin();
                        ofClear(0.0,0.0,0.0,0.0);
                        quadFbo.draw(0+quadDispX,0+quadDispY,quadW,quadH);
                        maskShader->end();
                        targetFbo.end();

                        targetFbo.getTextureReference().bind();

                        glMatrixMode(GL_TEXTURE);
                        glPushMatrix();//to scale the texture
                        glLoadIdentity();

                        ofTextureData texData = targetFbo.getTextureReference().getTextureData();
                        if(texData.textureTarget == GL_TEXTURE_RECTANGLE_ARB)
                        {
                            glScalef(targetFbo.getTextureReference().getWidth(), targetFbo.getTextureReference().getHeight(), 1.0f);
                        }
                        else
                        {
                            glScalef(targetFbo.getTextureReference().getWidth() / texData.tex_w, targetFbo.getTextureReference().getHeight() / texData.tex_h, 1.0f);
                        }
                        glMatrixMode(GL_MODELVIEW);
                        glEnable(GL_MAP2_VERTEX_3);
                        glEnable(GL_AUTO_NORMAL);
                        // this tries to prevent the double alpha problem
                        glEnable(GL_BLEND);
                        glBlendFunc(GL_ONE, GL_ONE_MINUS_SRC_ALPHA);
                        //draw the bezier shape
                        glEvalMesh2(GL_FILL, 0, 20, 0, 20);
                        glDisable(GL_BLEND);
                        glDisable(GL_MAP2_VERTEX_3);
                        glDisable(GL_AUTO_NORMAL);

                        targetFbo.getTextureReference().unbind();
                        glMatrixMode(GL_TEXTURE);
                        glPopMatrix();// texture scale pop matrix
                        glMatrixMode(GL_MODELVIEW);
                    }

                    if(bBlendModes)
                    {
                        glDisable(GL_BLEND);
                    }
                    ofDisableAlphaBlending();

                }
            }

            else
            {
                if(quadFbo.getWidth()>0)
                {
                    ofEnableAlphaBlending();
                    //set ofColor to white
                    ofSetColor(255,255,255);
                    //Blend modes stuff (with shaders would be better, but it scales bad on older GPUs)
                    if(bBlendModes)
                    {
                        glEnable(GL_BLEND);
                        if(blendMode == 0) glBlendFunc(GL_ONE, GL_ONE_MINUS_SRC_COLOR); //screen
                        else if(blendMode == 1) glBlendFunc(GL_ONE, GL_ONE); //add
                        else if(blendMode == 2) glBlendFunc(GL_DST_COLOR, GL_SRC_COLOR); // subtract
                        else if(blendMode == 3) glBlendFunc(GL_DST_COLOR, GL_ONE_MINUS_SRC_ALPHA); // multiply
                    }

                    if(!bDeform)
                    {
                        quadFbo.draw(0+quadDispX,0+quadDispY,quadW,quadH);
                    }
                    else
                    {
                        quadFbo.getTextureReference().bind();

                        glMatrixMode(GL_TEXTURE);
                        glPushMatrix();//to scale the texture
                        glLoadIdentity();

                        ofTextureData texData = quadFbo.getTextureReference().getTextureData();
                        if(texData.textureTarget == GL_TEXTURE_RECTANGLE_ARB)
                        {
                            glScalef(quadFbo.getTextureReference().getWidth(), quadFbo.getTextureReference().getHeight(), 1.0f);
                        }
                        else
                        {
                            glScalef(quadFbo.getTextureReference().getWidth() / texData.tex_w, quadFbo.getTextureReference().getHeight() / texData.tex_h, 1.0f);
                        }
                        glMatrixMode(GL_MODELVIEW);
                        //draw the bezier shape
                        glEnable(GL_MAP2_VERTEX_3);
                        glEnable(GL_AUTO_NORMAL);

                        glEvalMesh2(GL_FILL, 0, 20, 0, 20);

                        glDisable(GL_MAP2_VERTEX_3);
                        glDisable(GL_AUTO_NORMAL);

                        quadFbo.getTextureReference().unbind();
                        glMatrixMode(GL_TEXTURE);
                        glPopMatrix();// texture scale pop matrix
                        glMatrixMode(GL_MODELVIEW);
                    }

                    if(bBlendModes)
                    {
                        glDisable(GL_BLEND);
                    }
                    ofDisableAlphaBlending();
                }
            }
        }


        // draw mask hatch if we are in mask setup mode
        if(isActive && isMaskSetup)
        {
            ofEnableAlphaBlending();
            //set ofColor to red with alpha
            //ofSetColor(255,100,100,180);
            ofSetColor(100,139,150,160);
            maskFbo.draw(0+quadDispX,0+quadDispY,quadW,quadH);
            ofDisableAlphaBlending();
        }

        // draws a little triangle to highlight draggable corner
        if(isActive && bHighlightCorner && highlightedCorner >= 0)
        {
            ofFill();
            ofSetHexColor(0x00FF00);
            ofEnableAlphaBlending();
            if(highlightedCorner == 0)
            {
                ofTriangle(0,0,20,0,0,20);
            }
            else if(highlightedCorner == 1)
            {
                ofTriangle(ofGetWidth(),0,ofGetWidth()-20,0,ofGetWidth(),20);
            }
            else if(highlightedCorner == 2)
            {
                ofTriangle(ofGetWidth(),ofGetHeight(),ofGetWidth()-20,ofGetHeight(),ofGetWidth(),ofGetHeight()-20);
            }
            else if(highlightedCorner == 3)
            {
                ofTriangle(0,ofGetHeight(),0,ofGetHeight()-20,20,ofGetHeight());
            }
            ofDisableAlphaBlending();
            ofNoFill();
        }

        //lets draw a bounding box if we are in setup mode
        ofNoFill();
        ofEnableSmoothing();
        ofSetLineWidth(0.5);
        if (isSetup)
        {
            ofSetHexColor(borderColor);
            ofRect(0, 0, ofGetWidth(), ofGetHeight());
            // draws helper grid on active quad
            if (isActive)
            {
                ofSetHexColor(0x444444);
                ofLine(0,ofGetHeight()/2,ofGetWidth(),ofGetHeight()/2);
                ofLine(ofGetWidth()/2,0,ofGetWidth()/2,ofGetHeight());
                ofLine(ofGetWidth()/2,0,ofGetWidth()/2-20,20);
                ofLine(ofGetWidth()/2,0,ofGetWidth()/2+20,20);
                ofLine(ofGetWidth()/2-20,20,ofGetWidth()/2+20,20);
                ofLine(0,ofGetHeight()/4,ofGetWidth(),ofGetHeight()/4);
                ofLine(0,ofGetHeight()/2+ofGetHeight()/4,ofGetWidth(),ofGetHeight()/2+ofGetHeight()/4);
                ofLine(ofGetWidth()/4,0,ofGetWidth()/4,ofGetHeight());
                ofLine(ofGetWidth()/2+ofGetWidth()/4,0,ofGetWidth()/2+ofGetWidth()/4,ofGetHeight());
            }
        }

        ofPopMatrix();

        // draws markers for bezier deform setup
        if (isActive && isBezierSetup)
        {
            if (bBezier) {drawBezierMarkers();}
            else if (bGrid) {drawGridMarkers();}
        }

        // draws mask markers and contour in mask-setup mode
        if (isActive && isMaskSetup)
        {
            drawMaskMarkers();
        }


        if (isSetup)
        {
            ofSetHexColor(0x000000);
            ttf.drawString("surface "+ofToString(quadNumber), center.x*ofGetWidth(), center.y*ofGetHeight());
            if (isActive)
            {
                ofSetHexColor(0xDB6800);    // draws orange label if active quad, white if not
            }
            else
            {
                ofSetHexColor(0xFFFFFF);
            }
            ttf.drawString("surface "+ofToString(quadNumber), (center.x*ofGetWidth())-2, (center.y*ofGetHeight())-2);
        }

    }
}

//--------------------------------------------------------------
void quad::allocateFbo(int w, int h)
{
    settings.width = w;
    settings.height = h;
    quadFbo.allocate(settings);
    maskFbo.allocate(settings);
    targetFbo.allocate(settings);
}

//--------------------------------------------------------------
void quad::maskAddPoint(int x, int y)
{

    ofPoint mouse;
    mouse.x = x;
    mouse.y = y;

    ofVec3f warped;
    warped = findWarpedPoint(src, dst, mouse);

    maskPoints.push_back(warped);

}

ofVec3f quad::getWarpedPoint(int x, int y)
{
    ofPoint punto;
    punto.x = x;
    punto.y = y;

    ofVec3f warped;
    warped = findWarpedPoint(src, dst, punto);
    return warped;
}



//---------------------------------------------------------------
void quad::loadImageFromFile(string imgName, string imgPath)
{
    ofFile image(imgPath);
    imgBg = true;
    img.loadImage(image);
    bgImg = imgPath;
    loadedImg = imgName;
}


//---------------------------------------------------------------
void quad::loadVideoFromFile(string videoName, string videoPath)
{
    //ofFile movie(videoPath);
    videoBg = true;
    bgVideo = videoPath;
    loadedVideo = videoName;
    if (video.isLoaded())
    {
        video.closeMovie();
    }
    video.loadMovie(videoPath);
    videoWidth = video.width;
    videoHeight = video.height;
    if (videoTex.bAllocated())
    {
        videoTex.clear();
    }
    videoTex.allocate(videoWidth, videoHeight, GL_RGBA);
    videoAlphaPixels = new unsigned char [videoWidth*videoHeight*4];
    video.play();
    loadedVideo = videoName;
}

// Bezier helpers --------------------------------------
// Bezier setup -------------------------------------
void quad::bezierSurfaceSetup()
{
    bezierPoints =
    {
        {   {0, 0, 0},          {0.333, 0, 0},    {0.667, 0, 0},    {1.0, 0, 0}    },
        {   {0, 0.333, 0},        {0.333, 0.333, 0},  {0.667, 0.333, 0},  {1.0, 0.333, 0}  },
        {   {0, 0.667, 0},        {0.333, 0.667, 0},  {0.667, 0.667, 0},  {1.0, 0.667, 0}  },
        {   {0, 1.0, 0},        {0.333, 1.0, 0},  {0.667, 1.0, 0},  {1.0, 1.0, 0}  }
    };

    //This sets up my Bezier Surface
    bezierCtrlPoints =
    {
        {   {bezierPoints[0][0][0]*ofGetWidth(), bezierPoints[0][0][1]*ofGetHeight(), 0}, {bezierPoints[0][1][0]*ofGetWidth(), bezierPoints[0][1][1]*ofGetHeight(), 0}, {bezierPoints[0][2][0]*ofGetWidth(), bezierPoints[0][2][1]*ofGetHeight(), 0}, {bezierPoints[0][3][0]*ofGetWidth(), bezierPoints[0][3][1]*ofGetHeight(), 0} },
        {   {bezierPoints[1][0][0]*ofGetWidth(), bezierPoints[1][0][1]*ofGetHeight(), 0}, {bezierPoints[1][1][0]*ofGetWidth(), bezierPoints[1][1][1]*ofGetHeight(), 0}, {bezierPoints[1][2][0]*ofGetWidth(), bezierPoints[1][2][1]*ofGetHeight(), 0}, {bezierPoints[1][3][0]*ofGetWidth(), bezierPoints[1][3][1]*ofGetHeight(), 0}  },
        {   {bezierPoints[2][0][0]*ofGetWidth(), bezierPoints[2][0][1]*ofGetHeight(), 0}, {bezierPoints[2][1][0]*ofGetWidth(), bezierPoints[2][1][1]*ofGetHeight(), 0}, {bezierPoints[2][2][0]*ofGetWidth(), bezierPoints[2][2][1]*ofGetHeight(), 0}, {bezierPoints[2][3][0]*ofGetWidth(), bezierPoints[2][3][1]*ofGetHeight(), 0}  },
        {   {bezierPoints[3][0][0]*ofGetWidth(), bezierPoints[3][0][1]*ofGetHeight(), 0}, {bezierPoints[3][1][0]*ofGetWidth(), bezierPoints[3][1][1]*ofGetHeight(), 0}, {bezierPoints[3][2][0]*ofGetWidth(), bezierPoints[3][2][1]*ofGetHeight(), 0}, {bezierPoints[3][3][0]*ofGetWidth(), bezierPoints[3][3][1]*ofGetHeight(), 0}  }
    };

    //This sets up my Texture Surface
    GLfloat texpts [2][2][2] =
    {
        { {0, 0}, {1, 0} }, { {0, 1}, {1, 1} }
    };

    // enable depth test, so we only see the front
    glEnable(GL_DEPTH_TEST);
    //set up bezier surface with a 4th order evaluator
    glMap2f(GL_MAP2_VERTEX_3, 0, 1, 3, 4, 0, 1, 12, 4, &bezierCtrlPoints[0][0][0]);
    //set up texture map for bezier surface
    glMap2f(GL_MAP2_TEXTURE_COORD_2, 0, 1, 2, 2, 0, 1, 4, 2, &texpts[0][0][0]);
    glEnable(GL_MAP2_TEXTURE_COORD_2);
    glEnable(GL_MAP2_VERTEX_3);
    glEnable(GL_AUTO_NORMAL);
    glMapGrid2f(20, 0, 1, 20, 0, 1);
    glShadeModel(GL_FLAT);
}

// Bezier markers ----------------------------------------------
void quad::drawBezierMarkers()
{
    ofSetColor(220,200,0,255);
    ofSetLineWidth(1.5);
    for(unsigned int i = 0; i < 4; i++)
    {
        for(unsigned int j = 0; j < 4; j++)
        {
            ofVec3f punto;
            punto.x = bezierCtrlPoints[i][j][0];
            punto.y = bezierCtrlPoints[i][j][1];
            punto.z = bezierCtrlPoints[i][j][2];
            punto = findWarpedPoint(dst, src, punto);

            if(bHighlightCtrlPoint && highlightedCtrlPointRow == i && highlightedCtrlPointCol == j)
            {
                ofFill();
            }
            ofCircle(punto.x, punto.y, 3.6);
            ofNoFill();
        }
    }
    ofSetLineWidth(1.2);
    ofVec3f puntoA;
    ofVec3f puntoB;
    //
    puntoA.x = bezierCtrlPoints[0][0][0];
    puntoA.y = bezierCtrlPoints[0][0][1];
    puntoA.z = bezierCtrlPoints[0][0][2];
    puntoB.x = bezierCtrlPoints[0][1][0];
    puntoB.y = bezierCtrlPoints[0][1][1];
    puntoB.z = bezierCtrlPoints[0][1][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[0][0][0];
    puntoA.y = bezierCtrlPoints[0][0][1];
    puntoA.z = bezierCtrlPoints[0][0][2];
    puntoB.x = bezierCtrlPoints[1][0][0];
    puntoB.y = bezierCtrlPoints[1][0][1];
    puntoB.z = bezierCtrlPoints[1][0][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[0][0][0];
    puntoA.y = bezierCtrlPoints[0][0][1];
    puntoA.z = bezierCtrlPoints[0][0][2];
    puntoB.x = bezierCtrlPoints[1][1][0];
    puntoB.y = bezierCtrlPoints[1][1][1];
    puntoB.z = bezierCtrlPoints[1][1][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[0][3][0];
    puntoA.y = bezierCtrlPoints[0][3][1];
    puntoA.z = bezierCtrlPoints[0][3][2];
    puntoB.x = bezierCtrlPoints[1][3][0];
    puntoB.y = bezierCtrlPoints[1][3][1];
    puntoB.z = bezierCtrlPoints[1][3][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[0][3][0];
    puntoA.y = bezierCtrlPoints[0][3][1];
    puntoA.z = bezierCtrlPoints[0][3][2];
    puntoB.x = bezierCtrlPoints[0][2][0];
    puntoB.y = bezierCtrlPoints[0][2][1];
    puntoB.z = bezierCtrlPoints[0][2][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[0][3][0];
    puntoA.y = bezierCtrlPoints[0][3][1];
    puntoA.z = bezierCtrlPoints[0][3][2];
    puntoB.x = bezierCtrlPoints[1][2][0];
    puntoB.y = bezierCtrlPoints[1][2][1];
    puntoB.z = bezierCtrlPoints[1][2][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[3][0][0];
    puntoA.y = bezierCtrlPoints[3][0][1];
    puntoA.z = bezierCtrlPoints[3][0][2];
    puntoB.x = bezierCtrlPoints[3][1][0];
    puntoB.y = bezierCtrlPoints[3][1][1];
    puntoB.z = bezierCtrlPoints[3][1][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[3][0][0];
    puntoA.y = bezierCtrlPoints[3][0][1];
    puntoA.z = bezierCtrlPoints[3][0][2];
    puntoB.x = bezierCtrlPoints[2][0][0];
    puntoB.y = bezierCtrlPoints[2][0][1];
    puntoB.z = bezierCtrlPoints[2][0][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[3][0][0];
    puntoA.y = bezierCtrlPoints[3][0][1];
    puntoA.z = bezierCtrlPoints[3][0][2];
    puntoB.x = bezierCtrlPoints[2][1][0];
    puntoB.y = bezierCtrlPoints[2][1][1];
    puntoB.z = bezierCtrlPoints[2][1][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[3][3][0];
    puntoA.y = bezierCtrlPoints[3][3][1];
    puntoA.z = bezierCtrlPoints[3][3][2];
    puntoB.x = bezierCtrlPoints[3][2][0];
    puntoB.y = bezierCtrlPoints[3][2][1];
    puntoB.z = bezierCtrlPoints[3][2][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[3][3][0];
    puntoA.y = bezierCtrlPoints[3][3][1];
    puntoA.z = bezierCtrlPoints[3][3][2];
    puntoB.x = bezierCtrlPoints[2][3][0];
    puntoB.y = bezierCtrlPoints[2][3][1];
    puntoB.z = bezierCtrlPoints[2][3][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[3][3][0];
    puntoA.y = bezierCtrlPoints[3][3][1];
    puntoA.z = bezierCtrlPoints[3][3][2];
    puntoB.x = bezierCtrlPoints[2][2][0];
    puntoB.y = bezierCtrlPoints[2][2][1];
    puntoB.z = bezierCtrlPoints[2][2][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[1][2][0];
    puntoA.y = bezierCtrlPoints[1][2][1];
    puntoA.z = bezierCtrlPoints[1][2][2];
    puntoB.x = bezierCtrlPoints[2][2][0];
    puntoB.y = bezierCtrlPoints[2][2][1];
    puntoB.z = bezierCtrlPoints[2][2][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[1][2][0];
    puntoA.y = bezierCtrlPoints[1][2][1];
    puntoA.z = bezierCtrlPoints[1][2][2];
    puntoB.x = bezierCtrlPoints[1][1][0];
    puntoB.y = bezierCtrlPoints[1][1][1];
    puntoB.z = bezierCtrlPoints[1][1][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[2][1][0];
    puntoA.y = bezierCtrlPoints[2][1][1];
    puntoA.z = bezierCtrlPoints[2][1][2];
    puntoB.x = bezierCtrlPoints[1][1][0];
    puntoB.y = bezierCtrlPoints[1][1][1];
    puntoB.z = bezierCtrlPoints[1][1][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[2][1][0];
    puntoA.y = bezierCtrlPoints[2][1][1];
    puntoA.z = bezierCtrlPoints[2][1][2];
    puntoB.x = bezierCtrlPoints[2][2][0];
    puntoB.y = bezierCtrlPoints[2][2][1];
    puntoB.z = bezierCtrlPoints[2][2][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);

}

void quad::bezierSurfaceUpdate()
{
    // TODO: to optimize this try to limit recalculation to cases when it's really needed
    bezierCtrlPoints =
    {
        {   {bezierPoints[0][0][0]*ofGetWidth(), bezierPoints[0][0][1]*ofGetHeight(), 0}, {bezierPoints[0][1][0]*ofGetWidth(), bezierPoints[0][1][1]*ofGetHeight(), 0}, {bezierPoints[0][2][0]*ofGetWidth(), bezierPoints[0][2][1]*ofGetHeight(), 0}, {bezierPoints[0][3][0]*ofGetWidth(), bezierPoints[0][3][1]*ofGetHeight(), 0} },
        {   {bezierPoints[1][0][0]*ofGetWidth(), bezierPoints[1][0][1]*ofGetHeight(), 0}, {bezierPoints[1][1][0]*ofGetWidth(), bezierPoints[1][1][1]*ofGetHeight(), 0}, {bezierPoints[1][2][0]*ofGetWidth(), bezierPoints[1][2][1]*ofGetHeight(), 0}, {bezierPoints[1][3][0]*ofGetWidth(), bezierPoints[1][3][1]*ofGetHeight(), 0}  },
        {   {bezierPoints[2][0][0]*ofGetWidth(), bezierPoints[2][0][1]*ofGetHeight(), 0}, {bezierPoints[2][1][0]*ofGetWidth(), bezierPoints[2][1][1]*ofGetHeight(), 0}, {bezierPoints[2][2][0]*ofGetWidth(), bezierPoints[2][2][1]*ofGetHeight(), 0}, {bezierPoints[2][3][0]*ofGetWidth(), bezierPoints[2][3][1]*ofGetHeight(), 0}  },
        {   {bezierPoints[3][0][0]*ofGetWidth(), bezierPoints[3][0][1]*ofGetHeight(), 0}, {bezierPoints[3][1][0]*ofGetWidth(), bezierPoints[3][1][1]*ofGetHeight(), 0}, {bezierPoints[3][2][0]*ofGetWidth(), bezierPoints[3][2][1]*ofGetHeight(), 0}, {bezierPoints[3][3][0]*ofGetWidth(), bezierPoints[3][3][1]*ofGetHeight(), 0}  }
    };
    if(bBezier)
    {
        glMap2f(GL_MAP2_VERTEX_3, 0, 1, 3, 4, 0, 1, 12, 4, &bezierCtrlPoints[0][0][0]);
        GLfloat texpts [2][2][2] =
        {
            { {0, 0}, {1, 0} }, { {0, 1}, {1, 1} }
        };
        glMap2f(GL_MAP2_TEXTURE_COORD_2, 0, 1, 2, 2, 0, 1, 4, 2, &texpts[0][0][0]);
        glEnable(GL_MAP2_TEXTURE_COORD_2);
        glEnable(GL_MAP2_VERTEX_3);
        glEnable(GL_AUTO_NORMAL);
        glMapGrid2f(20, 0, 1, 20, 0, 1);
        glShadeModel(GL_FLAT);
    }

}

// Grid helpers --------------------------------------
// grid setup -------------------------------------
void quad::gridSurfaceSetup()
{

    for(int i=0; i<=gridRows; i++)
    {
        vector<vector<float> > row;
        for(int j=0; j<=gridColumns; j++)
        {
            vector<float> column;
            column.push_back((float)(1.0/gridColumns*j));
            column.push_back((float)(1.0/gridRows*i));
            column.push_back(0.0);
            row.push_back(column);
        }
        gridPoints.push_back(row);
    }

    gridCtrlPoints.clear();
    for(int i=0; i<=gridRows; i++)
    {

        for(int j=0; j<=gridColumns; j++)
        {
            gridCtrlPoints.push_back(gridPoints[i][j][0]*ofGetWidth());
            gridCtrlPoints.push_back(gridPoints[i][j][1]*ofGetHeight());
            gridCtrlPoints.push_back(0.0);
        }
    }

    GLfloat punti[gridRows+1][gridColumns+1][3];
    for(int i=0; i<=gridRows; i++)
    {
        for(int j=0; j<=gridColumns; j++)
        {
            punti[i][j][0] = gridPoints[i][j][0]*ofGetWidth();
            punti[i][j][1] = gridPoints[i][j][1]*ofGetHeight();
            punti[i][j][2] = 0.0;
        }
    }

    //This sets up my Texture Surface
    GLfloat texpts [2][2][2] =
    {
        { {0, 0}, {1, 0} }, { {0, 1}, {1, 1} }
    };

    // enable depth test, so we only see the front
    glEnable(GL_DEPTH_TEST);
    //set up bezier surface with a linear order evaluator
    //glMap2f(GL_MAP2_VERTEX_3, 0, 1, 3, (gridColumns+1), 0, 1, (gridColumns+1)*3, (gridRows+1), &punti[0][0][0]);
    glMap2f(GL_MAP2_VERTEX_3, 0, 1, 3, (gridColumns+1), 0, 1, (gridColumns+1)*3, (gridRows+1), &gridCtrlPoints[0]);
    //set up texture map for bezier surface
    glMap2f(GL_MAP2_TEXTURE_COORD_2, 0, 1, 2, 2, 0, 1, 4, 2, &texpts[0][0][0]);
    glEnable(GL_MAP2_TEXTURE_COORD_2);
    glEnable(GL_MAP2_VERTEX_3);
    glEnable(GL_AUTO_NORMAL);
    glMapGrid2f(20, 0, 1, 20, 0, 1);
    glShadeModel(GL_FLAT);
}


void quad::gridSurfaceUpdate()
{
    // TODO: to optimize this try to limit recalculation to cases when it's really needed
    //This sets up my Grid Surface

    if(gridPoints.size() != (gridRows+1) || gridPoints[0].size() != (gridColumns+1))
    {
        gridPoints.clear();
        for(int i=0; i<=gridRows; i++)
           {
               vector<vector<float> > row;
               for(int j=0; j<=gridColumns; j++)
                   {
                       vector<float> column;
                       column.push_back((float)(1.0/gridColumns*j));
                       column.push_back((float)(1.0/gridRows*i));
                       column.push_back(0.0);
                       row.push_back(column);
           }
           gridPoints.push_back(row);
           }
    }


    gridCtrlPoints.clear();
    for(int i=0; i<=gridRows; i++)
    {

        for(int j=0; j<=gridColumns; j++)
        {
            gridCtrlPoints.push_back(gridPoints[i][j][0]*ofGetWidth());
            gridCtrlPoints.push_back(gridPoints[i][j][1]*ofGetHeight());
            gridCtrlPoints.push_back(0.0);
        }
    }

/*
    GLfloat punti[gridRows+1][gridColumns+1][3];
    for(int i=0; i<=gridRows; i++)
    {
        for(int j=0; j<=gridColumns; j++)
        {
            punti[i][j][0] = gridPoints[i][j][0]*ofGetWidth();
            punti[i][j][1] = gridPoints[i][j][1]*ofGetHeight();
            punti[i][j][2] = 0.0;
        }
    }
*/

    GLfloat punti[(gridRows+1)*(gridColumns+1)*3];
    for(int i=0; i<=gridRows; i++)
    {
        for(int j=0; j<=gridColumns; j++)
        {
            punti[i*j+j] = gridPoints[i][j][0]*ofGetWidth();
            punti[i*j+j+1] = gridPoints[i][j][1]*ofGetHeight();
            punti[i*j+j+2] = 0.0;
        }
    }



    //glMap2f(GL_MAP2_VERTEX_3, 0, 1, 3, (gridColumns+1), 0, 1, (gridColumns+1)*3, (gridRows+1), &punti[0][0][0]);
    //glMap2f(GL_MAP2_VERTEX_3, 0, 1, 3, (gridColumns+1), 0, 1, (gridColumns+1)*3, (gridRows+1), &punti[0]);
    glMap2f(GL_MAP2_VERTEX_3, 0, 1, 3, (gridColumns+1), 0, 1, (gridColumns+1)*3, (gridRows+1), &gridCtrlPoints[0]);


    GLfloat texpts [2][2][2] =
    {
        { {0, 0}, {1, 0} }, { {0, 1}, {1, 1} }
    };
    glMap2f(GL_MAP2_TEXTURE_COORD_2, 0, 1, 2, 2, 0, 1, 4, 2, &texpts[0][0][0]);
    glEnable(GL_MAP2_TEXTURE_COORD_2);
    glEnable(GL_MAP2_VERTEX_3);
    glEnable(GL_AUTO_NORMAL);
    glMapGrid2f(20, 0, 1, 20, 0, 1);
    glShadeModel(GL_FLAT);


}


void quad::drawGridMarkers()
{
    ofSetColor(0,200,220,255);
    ofSetLineWidth(1.5);

    for(int i=0; i<=gridRows; i++)
    {
        for(int j=0; j<=gridColumns; j++)
        {
            ofVec3f punto;
            punto.x = gridPoints[i][j][0]*ofGetWidth();
            punto.y = gridPoints[i][j][1]*ofGetHeight();
            punto.z = 0.0;
            punto = findWarpedPoint(dst, src, punto);
            if(bHighlightCtrlPoint && highlightedCtrlPointRow == i && highlightedCtrlPointCol == j)
            {
                ofFill();
            }
            ofCircle(punto.x, punto.y, 3.0);
            ofNoFill();
        }
    }
}





// Mask helpers --------------------------------------
void quad::drawMaskMarkers()
{
    if (maskPoints.size()>0)
    {
        ofPolyline contour;
        for(unsigned int i = 0; i < maskPoints.size(); i++)
        {
            ofVec3f contourPoint;
            contourPoint.x = maskPoints[i].x;
            contourPoint.y = maskPoints[i].y;
            contourPoint.z = 0;
            contourPoint = findWarpedPoint(dst, src, contourPoint);
            contour.addVertex(contourPoint);
        }
        ofSetLineWidth(1.6);
        contour.close();
        contour.draw();

        for(unsigned int i = 0; i < maskPoints.size(); i++)
        {
            ofVec3f punto;
            punto.x = maskPoints[i].x;
            punto.y = maskPoints[i].y;
            punto.z = 0.0;
            punto = findWarpedPoint(dst, src, punto);
            ofSetColor(100,139,150,255);
            ofSetLineWidth(1.0);
            if(bHighlightMaskPoint && highlightedMaskPoint == i)
            {
                ofFill();
            }
            ofCircle(punto.x, punto.y, 4);
            ofNoFill();
            ofCircle(punto.x, punto.y, 10);
        }
    }
}
