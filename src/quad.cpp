#include "quad.h"

#include <sys/types.h>

#include <errno.h>
#include <vector>
#include <string>

#ifdef WITH_KINECT
    #ifdef WITH_SYPHON
    void quad::setup(float x1, float y1, float x2, float y2, float x3, float y3, float x4, float y4, ofShader &edgeBlendShader, ofShader &quadMaskShader, ofShader &chromaShader, vector<ofVideoGrabber> &cameras, vector<ofVideoPlayer> &sharedVideos, kinectManager &kinect, ofxSyphonClient &syphon)
    #else
    void quad::setup(float x1, float y1, float x2, float y2, float x3, float y3, float x4, float y4, ofShader &edgeBlendShader, ofShader &quadMaskShader, ofShader &chromaShader, vector<ofVideoGrabber> &cameras, vector<ofVideoPlayer> &sharedVideos, kinectManager &kinect)
    #endif
#else
    #ifdef WITH_SYPHON
    void quad::setup(float x1, float y1, float x2, float y2, float x3, float y3, float x4, float y4, ofShader &edgeBlendShader, ofShader &quadMaskShader, ofShader &chromaShader, vector<ofVideoGrabber> &cameras, vector<ofVideoPlayer> &sharedVideos, ofxSyphonClient &syphon)
    #else
    void quad::setup(float x1, float y1, float x2, float y2, float x3, float y3, float x4, float y4, ofShader &edgeBlendShader, ofShader &quadMaskShader, ofShader &chromaShader, vector<ofVideoGrabber> &cameras, vector<ofVideoPlayer> &sharedVideos)
    #endif
#endif
{

    shaderBlend = &edgeBlendShader;
    maskShader = &quadMaskShader;
    greenscreenShader = &chromaShader;
    //camera = &camGrabber;
    #ifdef WITH_KINECT
    quadKinect = &kinect;
    #endif
    #ifdef WITH_SYPHON
    syphClientTex = &syphon;
    #endif
    vids = sharedVideos;
    cams = cameras;
    if(cams.size()>0)
    {
        camAvailable = true;
    }
    else
    {
        camAvailable = false;
    }

    //loads load in some truetype fonts
    //ttf.loadFont("type/frabk.ttf", 11);
    ttf.loadFont("type/OpenSans-Regular.ttf", 11);


    bgImg = string("");
    loadedImg = string("");
    loadedVideo = string("");
    slideshowName = string("");
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

    crop[0] = 0.0;
    crop[1] = 0.0;
    crop[2] = 0.0;
    crop[3] = 0.0;

    circularCrop[0] = 0.5;
    circularCrop[1] = 0.5;
    circularCrop[2] = 0.0;

    //videos = videoFiles;
    //slideshows = slideshowFolders;

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
    sharedVideoBg = false;
    sharedVideoNum = 1;
    sharedVideoId = sharedVideoNum -1;
    slideshowBg = False;
    slideFit = False;
    slideKeepAspect = True;

    videoHFlip = False;
    imgHFlip = False;
    camHFlip = False;
    videoVFlip = False;
    imgVFlip = False;
    camVFlip = False;

    camNumber = prevCamNumber = 0;
    camMultX = 1;
    camMultY = 1;
    if (camAvailable)
    {
        setupCamera();
    }

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

    #ifdef WITH_SYPHON
    // syphon variables
	bSyphon = false;
	syphonPosX = 0.0;
	syphonPosY = 0.0;
	syphonScaleX = 1.0;
	syphonScaleY = 1.0;
	#endif

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
    quadFbo.allocate(settings);
    maskFbo.allocate(settings);
    targetFbo.allocate(settings);

    quadDispX = 0;
    quadDispY = 0;
    quadW = ofGetWidth();
    quadH = ofGetHeight();

    bBlendModes = False;
    blendMode = 0;

    bHighlightCorner = False;
    highlightedCorner = -1;

    bHighlightCenter = false;
    bHighlightRotation = false;

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
    gridColumns = 7;
    gridRows = 6;
    gridSurfaceSetup();

    timelineRed = 1.0;
    timelineGreen = 1.0;
    timelineBlu = 1.0;
    timelineAlpha = 1.0;

    bTimelineTint = false;
    bTimelineColor = false;
    bTimelineAlpha = false;
    bTimelineSlideChange = false;

}


void quad::update()
{
    if (isOn)
    {
        sharedVideoId = sharedVideoNum - 1;

        if(camAvailable && camNumber != prevCamNumber)
        {
            setupCamera();
            prevCamNumber = camNumber;
        }
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
            // if a different slideshow has been chosen in gui we do load its images
            if ((slideshowName != loadedSlideshow) && slideshowName != "")
            {
                // we exclude "." and ".." directories if present
                if (slideshowName != "." && slideshowName != "..")
                {
                    // we scan the img dir for images
                    string slidesDir = slideshowName;
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
                            slide.loadImage(slideshowName+"/"+slidesnames[i]);
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
        #ifdef WITH_KINECT
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
        #endif

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
        //calculates screen ratio factor for window and fullscreen
        float screenFactorX = (ofGetWidth()/(float)ofGetScreenWidth());
        float screenFactorY = (ofGetHeight()/(float)ofGetScreenHeight());
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
                ofSetColor(transColor.r * 255 * timelineRed, transColor.g * 255 * timelineGreen, transColor.b * 255 * timelineBlu, transColor.a * 255 * timelineAlpha);
            }
            // this in case of only one color set
            else
            {
                ofSetColor(bgColor.r * 255 * timelineRed, bgColor.g * 255 * timelineGreen, bgColor.b * 255 * timelineBlu, bgColor.a * 255 * timelineAlpha);
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
            ofSetColor(videoColorize.r * 255 * timelineRed, videoColorize.g * 255 * timelineGreen, videoColorize.b * 255 * timelineBlu, videoColorize.a * 255 * timelineAlpha);
            if (!videoLoop)
            {
                // in no-looping mode it stops drawing video frame when video reaches the end
                // using 'getIsMovieDone()' because there are problems with getting head position under GStream
                if (!video.getIsMovieDone())
                {
                    if (videoGreenscreen)
                    {
                        greenscreenShader->begin();
                        greenscreenShader->setUniformTexture("tex", video.getTextureReference(),0 );
                        greenscreenShader->setUniform1f("greenscreenR", colorGreenscreen.r);
                        greenscreenShader->setUniform1f("greenscreenG", colorGreenscreen.g);
                        greenscreenShader->setUniform1f("greenscreenB", colorGreenscreen.b);
                        // we pass tint values too
                        greenscreenShader->setUniform1f("tintR", videoColorize.r);
                        greenscreenShader->setUniform1f("tintG", videoColorize.g);
                        greenscreenShader->setUniform1f("tintB", videoColorize.b);
                        greenscreenShader->setUniform1f("greenscreenT", (float)thresholdGreenscreen/255.0);
                        video.draw(0,0,videoWidth*videoMultX, videoHeight*videoMultY);
                        greenscreenShader->end();
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
                        greenscreenShader->begin();
                        greenscreenShader->setUniformTexture("tex", video.getTextureReference(),0 );
                        greenscreenShader->setUniform1f("greenscreenR", colorGreenscreen.r);
                        greenscreenShader->setUniform1f("greenscreenG", colorGreenscreen.g);
                        greenscreenShader->setUniform1f("greenscreenB", colorGreenscreen.b);
                        // we pass tint values too
                        greenscreenShader->setUniform1f("tintR", videoColorize.r);
                        greenscreenShader->setUniform1f("tintG", videoColorize.g);
                        greenscreenShader->setUniform1f("tintB", videoColorize.b);
                        greenscreenShader->setUniform1f("greenscreenT", (float)thresholdGreenscreen/255.0);
                        if(video.isPlaying())
                        {
                            video.draw(0,0,videoWidth*videoMultX, videoHeight*videoMultY);
                        }
                        greenscreenShader->end();
                }
                else
                {
                    if(video.isPlaying())
                    {
                        video.draw(0,0,videoWidth*videoMultX, videoHeight*videoMultY);
                    }
                }
            }
            if (videoHFlip || videoVFlip)
            {
                glPopMatrix();
            }
        }

        // shared video ----------------------------------------------------------------------

        if (sharedVideoBg)
        {
            if (videoHFlip || videoVFlip)
            {
                glPushMatrix();
                if(videoHFlip && !videoVFlip)
                {
                    ofTranslate(vids[sharedVideoId].width*videoMultX,0);
                    glScalef(-1,1,1);
                }
                else if(videoVFlip && !videoHFlip)
                {
                    ofTranslate(0,vids[sharedVideoId].height*videoMultY);
                    glScalef(1,-1,1);
                }
                else
                {
                    ofTranslate(vids[sharedVideoId].width*videoMultX,vids[sharedVideoId].height*videoMultY);
                    glScalef(-1,-1,1);
                }
            }
            ofSetColor(videoColorize.r * 255 * timelineRed, videoColorize.g * 255 * timelineGreen, videoColorize.b * 255 * timelineBlu, videoColorize.a * 255 * timelineAlpha);
            if (videoGreenscreen)
            {
                greenscreenShader->begin();
                greenscreenShader->setUniformTexture("tex", vids[sharedVideoId].getTextureReference(),0 );
                greenscreenShader->setUniform1f("greenscreenR", colorGreenscreen.r);
                greenscreenShader->setUniform1f("greenscreenG", colorGreenscreen.g);
                greenscreenShader->setUniform1f("greenscreenB", colorGreenscreen.b);
                // we pass tint values too
                greenscreenShader->setUniform1f("tintR", videoColorize.r);
                greenscreenShader->setUniform1f("tintG", videoColorize.g);
                greenscreenShader->setUniform1f("tintB", videoColorize.b);
                greenscreenShader->setUniform1f("greenscreenT", (float)thresholdGreenscreen/255.0);
                vids[sharedVideoId].draw(0,0,vids[sharedVideoId].width*videoMultX, vids[sharedVideoId].height*videoMultY);
                greenscreenShader->end();
            }
            else
            {
                vids[sharedVideoId].draw(0,0,vids[sharedVideoId].width*videoMultX, vids[sharedVideoId].height*videoMultY);
            }
            if (videoHFlip || videoVFlip)
            {
                glPopMatrix();
            }
        }


        // camera ------------------------------------------------------------------------------
        // camera stuff
        if (camAvailable && camBg && cams[camNumber].width > 0)
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
            ofSetColor(camColorize.r * 255 * timelineRed, camColorize.g * 255 * timelineGreen, camColorize.b * 255 * timelineBlu, camColorize.a * 255 * timelineAlpha);
            if (camGreenscreen)
            {
                greenscreenShader->begin();
                greenscreenShader->setUniformTexture("tex", cams[camNumber].getTextureReference(),0 );
                greenscreenShader->setUniform1f("greenscreenR", colorGreenscreen.r);
                greenscreenShader->setUniform1f("greenscreenG", colorGreenscreen.g);
                greenscreenShader->setUniform1f("greenscreenB", colorGreenscreen.b);
                // we pass tint values too
                greenscreenShader->setUniform1f("tintR", camColorize.r);
                greenscreenShader->setUniform1f("tintG", camColorize.g);
                greenscreenShader->setUniform1f("tintB", camColorize.b);
                greenscreenShader->setUniform1f("greenscreenT", (float)thresholdGreenscreen/255.0);
                cams[camNumber].getTextureReference().draw(0,0,camWidth*camMultX,camHeight*camMultY);
                greenscreenShader->end();
            }
            else
            {
                //camTexture.draw(0,0,camWidth*camMultX,camHeight*camMultY); // orig
                cams[camNumber].getTextureReference().draw(0,0,camWidth*camMultX,camHeight*camMultY);

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
                ofSetColor(imgColorize.r * 255 * timelineRed, imgColorize.g * 255 * timelineGreen, imgColorize.b * 255 * timelineBlu, imgColorize.a * 255 * timelineAlpha);
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
                    // check if we are using timeline to change slides
                    if(!bTimelineSlideChange)
                    {
                        currentSlide += 1;
                    }
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
                    ofTranslate(img.getWidth()*imgMultX*screenFactorX,0);
                    glScalef(-1,1,1);
                }
                else if(imgVFlip && !imgHFlip)
                {
                    ofTranslate(0,img.getHeight()*imgMultY*screenFactorY);
                    glScalef(1,-1,1);
                }
                else
                {
                    ofTranslate(img.getWidth()*imgMultX*screenFactorX,img.getHeight()*imgMultY*screenFactorY);
                    glScalef(-1,-1,1);
                }
            }
            ofSetColor(imgColorize.r * 255 * timelineRed, imgColorize.g * 255 * timelineGreen, imgColorize.b * 255 * timelineBlu, imgColorize.a * 255 * timelineAlpha);
            //img.draw(0,0,img.getWidth()*imgMultX, img.getHeight()*imgMultY);
            //img.draw(0,0,img.getWidth()*imgMultX/(img.getWidth()/ofGetWidth()), img.getHeight()*imgMultY/(img.getHeight()/ofGetHeight()));
            img.draw(0,0,img.getWidth()*imgMultX*screenFactorX, img.getHeight()*imgMultY*screenFactorY);
            if (imgHFlip || imgVFlip)
            {
                glPopMatrix();
            }
        }

        // kinect stuff
        #ifdef WITH_KINECT
        if (kinectBg && kinectImg)
        {
            ofSetColor(kinectColorize.r * 255 * timelineRed, kinectColorize.g * 255 * timelineGreen, kinectColorize.b * 255 * timelineBlu, kinectColorize.a * 255 * timelineAlpha);
            //quadKinect->grayImage.draw(0,0,quadKinect->grayImage.getWidth()*kinectMultX,quadKinect->grayImage.getHeight()*kinectMultY);
            if (getKinectContours)
            {
                ofPushStyle();
                glPushMatrix();
                glScalef( kinectMultX, kinectMultY, 0.0 );
                // ----------- draw the kinect path made of detected blobs
                ofColor pathColor(kinectColorize.r * 255 * timelineRed, kinectColorize.g * 255 * timelineGreen, kinectColorize.b * 255 * timelineBlu, kinectColorize.a * 255 * timelineAlpha);
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
        #endif

        #ifdef WITH_SYPHON
        // syphon stuff
		if (bSyphon)
		{
			ofSetColor(255, 255, 255);
			syphClientTex->draw(syphonPosX, syphonPosY, syphonScaleX*syphClientTex->getWidth(), syphonScaleY*syphClientTex->getHeight());
		}
        #endif

        ofDisableAlphaBlending();
        quadFbo.end();

        //mask on mask FBO
        maskFbo.begin();
        ofClear(0.0,0.0,0.0,0.0);
        ofEnableAlphaBlending();
        ofFill();
        ofEnableSmoothing();

        // crop rectangular mask
        ofSetColor(255,255,255);
        ofRect(0,0,ofGetWidth(),crop[0]*ofGetHeight());
        ofRect(ofGetWidth()*(1-crop[1]),0,ofGetWidth()*crop[1],ofGetHeight());
        ofRect(0,ofGetHeight()*(1-crop[2]),ofGetWidth(),crop[2]*ofGetHeight());
        ofRect(0,0,ofGetWidth()*crop[3], ofGetHeight());

        // crop circular mask
        if(circularCrop[2]>0.0)
        {
            ofSetCircleResolution(64);
            ofCircle(circularCrop[0]*ofGetWidth(), circularCrop[1]*ofGetHeight(), circularCrop[2]*ofGetWidth());
            ofSetCircleResolution(22);
        }

        // user mask
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
        #ifdef WITH_KINECT
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
        #endif
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

                    if(bBezier)
                    {
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
                    else if(bGrid)
                    {
                        targetFbo.getTextureReference().bind();
                        gridMesh.drawFaces();
                        targetFbo.getTextureReference().unbind();
                        if(isActive && isBezierSetup)
                        {
                            ofPushStyle();
                            ofSetColor(255,255,255,200);
                            gridMesh.drawWireframe();
                            ofPopStyle();
                        }
                    }

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


                        if(bBezier)
                        {
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
                        else if(bGrid)
                        {
                            targetFbo.getTextureReference().bind();
                            gridMesh.drawFaces();
                            targetFbo.getTextureReference().unbind();
                            if(isActive && isBezierSetup)
                            {
                                ofPushStyle();
                                ofSetColor(255,255,255,200);
                                gridMesh.drawWireframe();
                                ofPopStyle();
                            }
                        }

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

                        if(bBezier)
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
                        else if(bGrid)
                        {
                            quadFbo.getTextureReference().bind();
                            gridMesh.drawFaces();
                            quadFbo.getTextureReference().unbind();
                            if(isActive && isBezierSetup)
                            {
                                ofPushStyle();
                                ofSetColor(255,255,255,200);
                                gridMesh.drawWireframe();
                                ofPopStyle();
                            }
                        }
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

        /*
        // draws a little triangle to highlight draggable corner
        if(isActive && bHighlightCorner && highlightedCorner >= 0)
        {
            ofFill();
            ofSetHexColor(0x00FF00);
            ofEnableAlphaBlending();
            if(highlightedCorner == 0)
            {
                ofTriangle(0,0,25,0,0,25);
            }
            else if(highlightedCorner == 1)
            {
                ofTriangle(ofGetWidth(),0,ofGetWidth()-25,0,ofGetWidth(),25);
            }
            else if(highlightedCorner == 2)
            {
                ofTriangle(ofGetWidth(),ofGetHeight(),ofGetWidth()-25,ofGetHeight(),ofGetWidth(),ofGetHeight()-25);
            }
            else if(highlightedCorner == 3)
            {
                ofTriangle(0,ofGetHeight(),0,ofGetHeight()-25,25,ofGetHeight());
            }
            ofDisableAlphaBlending();
            ofNoFill();
        }
        */

        //lets draw a bounding box if we are in setup mode
        ofNoFill();
        //ofEnableSmoothing();
        ofSetLineWidth(1.0);
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
            if (bBezier)
            {
                drawBezierMarkers();
            }
            else if (bGrid)
            {
                drawGridMarkers();
            }
        }

        // draws mask markers and contour in mask-setup mode
        if (isActive && isMaskSetup)
        {
            drawMaskMarkers();
        }


        // draws a little circle to highlight draggable corner
        if(isActive && bHighlightCorner && highlightedCorner >= 0)
        {
            //ofFill();
            //ofSetColor(0,200,220,120);
            ofSetColor(219,104,0,255);
            ofEnableAlphaBlending();
            ofCircle(corners[highlightedCorner].x*ofGetWidth(),corners[highlightedCorner].y*ofGetHeight(),5);
            ofCircle(corners[highlightedCorner].x*ofGetWidth(),corners[highlightedCorner].y*ofGetHeight(),20);
            ofDisableAlphaBlending();
            //ofNoFill();
        }

        if (isSetup)
        {
            ofSetHexColor(0x000000);
            ttf.drawString("surface "+ofToString(quadNumber), center.x*ofGetWidth()+10, center.y*ofGetHeight()-10);
            if (isActive)
            {

                if(bHighlightCenter)
                {
                    ofFill();
                    ofEnableAlphaBlending();
                    //ofSetColor(0,200,220,120);
                    ofSetColor(219,104,0,120);
                    ofRect(center.x*ofGetWidth()-5,center.y*ofGetHeight()-5,10,10);
                    ofDisableAlphaBlending();
                    ofNoFill();
                    //ofSetColor(255,255,255,255);
                    ofLine(center.x*ofGetWidth()-25,center.y*ofGetHeight(),center.x*ofGetWidth()-15,center.y*ofGetHeight());
                    ofLine(center.x*ofGetWidth()-15,center.y*ofGetHeight(),center.x*ofGetWidth()-19,center.y*ofGetHeight()+4);
                    ofLine(center.x*ofGetWidth()-15,center.y*ofGetHeight(),center.x*ofGetWidth()-19,center.y*ofGetHeight()-4);
                    ofLine(center.x*ofGetWidth()-25,center.y*ofGetHeight(),center.x*ofGetWidth()-25,center.y*ofGetHeight()-10);
                    ofLine(center.x*ofGetWidth()-25,center.y*ofGetHeight()-10,center.x*ofGetWidth()-29,center.y*ofGetHeight()-6);
                    ofLine(center.x*ofGetWidth()-25,center.y*ofGetHeight()-10,center.x*ofGetWidth()-21,center.y*ofGetHeight()-6);

                    ofLine(center.x*ofGetWidth()-25,center.y*ofGetHeight(),center.x*ofGetWidth()-35,center.y*ofGetHeight());
                    ofLine(center.x*ofGetWidth()-35,center.y*ofGetHeight(),center.x*ofGetWidth()-31,center.y*ofGetHeight()+4);
                    ofLine(center.x*ofGetWidth()-35,center.y*ofGetHeight(),center.x*ofGetWidth()-31,center.y*ofGetHeight()-4);
                    ofLine(center.x*ofGetWidth()-25,center.y*ofGetHeight(),center.x*ofGetWidth()-25,center.y*ofGetHeight()+10);
                    ofLine(center.x*ofGetWidth()-25,center.y*ofGetHeight()+10,center.x*ofGetWidth()-29,center.y*ofGetHeight()+6);
                    ofLine(center.x*ofGetWidth()-25,center.y*ofGetHeight()+10,center.x*ofGetWidth()-21,center.y*ofGetHeight()+6);

                    ofSetCircleResolution(48);
                    ofCircle(center.x*ofGetWidth()-25,center.y*ofGetHeight(),12);
                    ofSetCircleResolution(40);
                    ofSetHexColor(0x444444);

                }

                if(bHighlightRotation)
                {
                    ofFill();
                    ofEnableAlphaBlending();
                    //ofSetColor(0,200,220,120);
                    ofSetColor(219,104,0,120);
                    ofCircle((center.x+0.1)*ofGetWidth(),center.y*ofGetHeight(),5);
                    ofDisableAlphaBlending();
                    ofSetHexColor(0x444444);
                    ofNoFill();
                }
                ofSetHexColor(0x444444);
                ofRect(center.x*ofGetWidth()-5,center.y*ofGetHeight()-5,10,10);
                ofCircle((center.x+0.1)*ofGetWidth(),center.y*ofGetHeight(),5);
                ofLine(center.x*ofGetWidth(),center.y*ofGetHeight(),(center.x+0.1)*ofGetWidth(),center.y*ofGetHeight());

                ofSetColor(219,104,0,255);    // draws orange label if active quad, white if not
                ttf.drawString("surface "+ofToString(quadNumber), (center.x*ofGetWidth())+8, (center.y*ofGetHeight())-12);
            }
            else
            {
                ofSetHexColor(0xFFFFFF);
                ttf.drawString("surface "+ofToString(quadNumber), (center.x*ofGetWidth())+8, (center.y*ofGetHeight())-12);
            }

        }

    }
}
