#include "quad.h"
#include "homography.h"

#include <sys/types.h>
#include <dirent.h>
#include <errno.h>
#include <vector>
#include <string>

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


void quad::setup(float x1, float y1, float x2, float y2, float x3, float y3, float x4, float y4, vector<string> &slideshowFolders, ofShader &edgeBlendShader, ofVideoGrabber &camGrabber)
{

    shaderBlend = &edgeBlendShader;
    camera = &camGrabber;

    // load shaders
    maskShader.load("shaders/mask.vert", "shaders/mask.frag");

    //loads load in some truetype fonts
    ttf.loadFont("type/frabk.ttf", 11);

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

    edgeBlendBool = False;
    edgeBlendExponent = 1.0;
    edgeBlendGamma = 1.8;
    edgeBlendAmountSin = 0.3;
    edgeBlendAmountDx = 0.3;

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

}


void quad::loadImageFromFile(string imgName, string imgPath)
{
    ofFile image(imgPath);
    imgBg = true;
    img.loadImage(image);
    bgImg = imgPath;
    loadedImg = imgName;
}

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


void quad::maskAddPoint(int x, int y)
{

    ofPoint mouse;
    mouse.x = x;
    mouse.y = y;

    ofVec3f warped;
    warped = findWarpedPoint(src, dst, mouse);

    maskPoints.push_back(warped);

    /*
    cout << "warped x = " << warped.x <<"\n";
    cout << "warped y = " << warped.y <<"\n";
    cout << "warped z = " << warped.z <<"\n\n";
    */
}


void quad::update()
{
    if (isOn)
    {
        //recalculates center of quad
        center = (corners[0]+corners[1]+corners[2]+corners[3])/4;

        //allocateFbo(ofGetWidth(), ofGetHeight());


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
        if (camBg)
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
        if (camBg)
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


        ofDisableAlphaBlending();
        quadFbo.end();

        //mask
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
            ofPolyline contour;
            for(unsigned int i = 0; i < maskPoints.size(); i++)
                {
                    contour.addVertex(maskPoints[i]);
                }
            ofSetLineWidth(1.2);
            contour.draw();
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
                shaderBlend->setUniform2f("amount", edgeBlendAmountSin, edgeBlendAmountDx);
                shaderBlend->setUniform1i("w", ofGetWidth());
                shaderBlend->setUniform1i("h", ofGetHeight());
                //set ofColor to white
                ofSetColor(255,255,255);
                //Blend modes stuff (with shaders would be better, but it scales bad on older GPUs)
                if(bBlendModes) {
                    glEnable(GL_BLEND);
                    if(blendMode == 0) glBlendFunc(GL_ONE, GL_ONE_MINUS_SRC_COLOR); //screen
                    else if(blendMode == 1) glBlendFunc(GL_ONE, GL_ONE); //add
                    else if(blendMode == 2) glBlendFunc(GL_DST_COLOR, GL_SRC_COLOR); // subtract
                    else if(blendMode == 3) glBlendFunc(GL_DST_COLOR, GL_ONE_MINUS_SRC_ALPHA); // multiply
                }
                quadFbo.draw(0+quadDispX,0+quadDispY,quadW,quadH);
                if(bBlendModes) {
                    glDisable(GL_BLEND);
                }
                ofDisableAlphaBlending();
                shaderBlend->end();
            }
        }

        else
        {

            if(bMask)
            {
                if(quadFbo.getWidth()>0)
                {
                    ofEnableAlphaBlending();
                    if (maskInvert) { maskMode = 1;} else { maskMode = 0;}
                    maskShader.begin();
                    maskShader.setUniformTexture ("tex", quadFbo.getTextureReference(), 0);
                    maskShader.setUniformTexture ("mask", maskFbo.getTextureReference(), 1);
                    maskShader.setUniform1i ("mode", maskMode);
                    //set ofColor to white
                    ofSetColor(255,255,255);
                    //Blend modes stuff (with shaders would be better, but it scales bad on older GPUs)
                    if(bBlendModes) {
                        glEnable(GL_BLEND);
                        if(blendMode == 0) glBlendFunc(GL_ONE, GL_ONE_MINUS_SRC_COLOR); //screen
                        else if(blendMode == 1) glBlendFunc(GL_ONE, GL_ONE); //add
                        else if(blendMode == 2) glBlendFunc(GL_DST_COLOR, GL_SRC_COLOR); // subtract
                        else if(blendMode == 3) glBlendFunc(GL_DST_COLOR, GL_ONE_MINUS_SRC_ALPHA); // multiply
                    }
                    quadFbo.draw(0+quadDispX,0+quadDispY,quadW,quadH);
                    if(bBlendModes) {
                        glDisable(GL_BLEND);
                    }
                    ofDisableAlphaBlending();
                    maskShader.end();
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
                    if(bBlendModes) {
                        glEnable(GL_BLEND);
                        if(blendMode == 0) glBlendFunc(GL_ONE, GL_ONE_MINUS_SRC_COLOR); //screen
                        else if(blendMode == 1) glBlendFunc(GL_ONE, GL_ONE); //add
                        else if(blendMode == 2) glBlendFunc(GL_DST_COLOR, GL_SRC_COLOR); // subtract
                        else if(blendMode == 3) glBlendFunc(GL_DST_COLOR, GL_ONE_MINUS_SRC_ALPHA); // multiply
                    }
                    quadFbo.draw(0+quadDispX,0+quadDispY,quadW,quadH);
                    if(bBlendModes) {
                        glDisable(GL_BLEND);
                    }
                    ofDisableAlphaBlending();
                }
            }
        }

        // draw mask
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
            if(highlightedCorner == 0) {ofTriangle(0,0,20,0,0,20);}
            else if(highlightedCorner == 1) {ofTriangle(ofGetWidth(),0,ofGetWidth()-20,0,ofGetWidth(),20);}
            else if(highlightedCorner == 2) {ofTriangle(ofGetWidth(),ofGetHeight(),ofGetWidth()-20,ofGetHeight(),ofGetWidth(),ofGetHeight()-20);}
            else if(highlightedCorner == 3) {ofTriangle(0,ofGetHeight(),0,ofGetHeight()-20,20,ofGetHeight());}
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

        if (isSetup)
        {
            ofSetHexColor(0x000000);
            ttf.drawString("surface "+ofToString(quadNumber), center.x*ofGetWidth(), center.y*ofGetHeight());
            if (isActive) { ofSetHexColor(0xDB6800); } // draws orange label if active quad, white if not
            else { ofSetHexColor(0xFFFFFF); }
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
}
