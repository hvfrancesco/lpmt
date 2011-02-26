#include "quad.h"


    // a func for reading a dir content to a vector of strings
    int quad::getdir (string dir, vector<string> &files)
    {
    DIR *dp;
    struct dirent *dirp;
    if((dp  = opendir(dir.c_str())) == NULL) {
        cout << "Error(" << errno << ") opening " << dir << endl;
        return errno;
    }

    while ((dirp = readdir(dp)) != NULL) {
        files.push_back(string(dirp->d_name));
    }
    closedir(dp);
    return 0;
    }


    void quad::setup(float x1, float y1, float x2, float y2, float x3, float y3, float x4, float y4, vector<string> &imgFiles, vector<string> &videoFiles, vector<string> &slideshowFolders)
    {

        //loads load in some truetype fonts
        ttf.loadFont("type/frabk.ttf", 22);
        ttf2.loadFont("type/frabk.ttf", 14);
        //lets load a test image too
        //img.loadImage("car.jpg");

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

        images = imgFiles;
        videos = videoFiles;
        slideshows = slideshowFolders;

        borderColor = 0x666666;

	    // sets default variables
        initialized = True;
        isSetup = True;
        isOn = True;
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

    }

    void quad::update()
    {
        if (isOn) {

        // images --------------------------------------------------------------------
        // loads an image if it has changed
        if (imgBg) {
            string imgName = images[bgImg];
            if (imgName != loadedImg) {
                img.loadImage("img/"+imgName);
                loadedImg = imgName;
            }
        }

        // solid colors ---------------------------------------------------------------
        // calculates transition between two solid colors
        if (colorBg && transBg) {
            if (transUp) {
            startColor = bgColor;
            endColor = secondColor;
            }
            else {
            startColor = secondColor;
            endColor = bgColor;
            }
            // using fps detected at setup is suboptimal
            // but updating it at each cycle triggers a flickering effect
            // needs more work (e.g. introducing a roundup of actual framerate)
            // now we update fps value just if it differs more than 50fps from actual rate
            //transStep = (transDuration * ofGetFrameRate());
            if (abs(fps-ofGetFrameRate()) > 50) {fps = ofGetFrameRate();}
            transStep = (transDuration * fps);
            transColor.r = startColor.r + (((endColor.r - startColor.r)/transStep)*transCounter);
            transColor.g = startColor.g + (((endColor.g - startColor.g)/transStep)*transCounter);
            transColor.b = startColor.b + (((endColor.b - startColor.b)/transStep)*transCounter);
            transColor.a = startColor.a + (((endColor.a - startColor.a)/transStep)*transCounter);
            transCounter += 1;
            if (transCounter >= transStep) {
                transCounter = 0;
                transUp = !transUp;
            }

        }


        // live camera --------------------------------------------------------------
        if (camBg) {
            if (camGreenscreen) {
            // checking for greenscreen color match
                for (int i = 0; i < camWidth*camHeight; i++) {
                    int deltaR = abs(camPixels[i*3+0] - colorGreenscreen.r*255);
                    int deltaG = abs(camPixels[i*3+1] - colorGreenscreen.g*255);
                    int deltaB = abs(camPixels[i*3+2] - colorGreenscreen.b*255);
                    if (deltaR <= thresholdGreenscreen && deltaG <= thresholdGreenscreen && deltaB <= thresholdGreenscreen) {
                        camAlphaPixels[i*4+3] = 0;
                    }
                    else {camAlphaPixels[i*4+3] = 255;}
                    // RGB data is copied untouched
                    camAlphaPixels[i*4+0] = camPixels[i*3+0];
                    camAlphaPixels[i*4+1] = camPixels[i*3+1];
                    camAlphaPixels[i*4+2] = camPixels[i*3+2];

                    }
                // loads data into texture with alpha
                camAlphaTexture.loadData(camAlphaPixels,camWidth,camHeight,GL_RGBA);
                }
            else {
                // loads camera pixels into this quad camera-texture wirh no alpha
                camTexture.loadData(camPixels, camWidth, camHeight, GL_RGB);
            }
        }


        // video --------------------------------------------------------------------
        // loads video
        if (videoBg) {
            string videoName = videos[bgVideo];
            if (videoName != loadedVideo) {
                if (video.bLoaded) { video.closeMovie(); }
                video.loadMovie("video/"+videoName);
                videoWidth = video.width;
                videoHeight = video.height;
                if (videoTex.bAllocated()) {videoTex.clear();}
                videoTex.allocate(videoWidth, videoHeight, GL_RGBA);
                videoAlphaPixels = new unsigned char [videoWidth*videoHeight*4];
                video.play();
                loadedVideo = videoName;
                }
            video.setVolume(videoVolume);
            // check for looping config parameter of video and sets loopstate - OF_LOOP_NORMAL = cycles / OF_LOOP_NONE = stops at the end
            if (videoLoop) {
                video.setLoopState(OF_LOOP_NORMAL);
                }
            else {
                video.setLoopState(OF_LOOP_NONE);
                }
            video.idleMovie();

            // video greenscreen stuff
            if (videoGreenscreen) {
            // gets video frame pixels array
            videoPixels = video.getPixels();
            // checking for greenscreen color match
            for (int i = 0; i < videoWidth*videoHeight; i++) {
                int deltaR = abs(videoPixels[i*3+0] - colorGreenscreen.r*255);
                int deltaG = abs(videoPixels[i*3+1] - colorGreenscreen.g*255);
                int deltaB = abs(videoPixels[i*3+2] - colorGreenscreen.b*255);
                if (deltaR <= thresholdGreenscreen && deltaG <= thresholdGreenscreen && deltaB <= thresholdGreenscreen) {
                    videoAlphaPixels[i*4+3] = 0;
                }
                else {videoAlphaPixels[i*4+3] = 255;}
                // RGB data is copied untouched
                videoAlphaPixels[i*4+0] = videoPixels[i*3+0];
                videoAlphaPixels[i*4+1] = videoPixels[i*3+1];
                videoAlphaPixels[i*4+2] = videoPixels[i*3+2];
                }
            videoTex.loadData(videoAlphaPixels,videoWidth,videoHeight,GL_RGBA);
            }

            // changevideo speed
            if (previousSpeed != videoSpeed) {
            video.setSpeed(videoSpeed);
            previousSpeed = videoSpeed;
            }
            }

        // slideshow -----------------------------------------------------------------
        if (slideshowBg) {
        // put it to off while loading images
        slideshowBg = False;
        string slideshowName = slideshows[bgSlideshow];
        // if a different slideshow has been chosen in gui we do load its images
        if (slideshowName != loadedSlideshow) {
            // we exclude "." and ".." directories if present
            if (slideshowName != "." && slideshowName != "..") {
            // we scan the img dir for images
            string slidesDir = string("./data/slideshow/");
            slidesDir += slideshowName;
            // make two arrays, one for imgs names and one for images
            slidesnames = vector<string>();
            slides = vector<ofImage>();
            // read all content of show folder
            getdir(slidesDir,slidesnames);
            // for each name found loads the image and populates the imgs array (excluding "." an "..")
            for (unsigned int i = 0;i < slidesnames.size();i++) {
                if (slidesnames[i] != "." && slidesnames[i] != "..") {
                    slide.loadImage("slideshow/"+slideshowName+"/"+slidesnames[i]);
                    slides.push_back(slide);
                }
            }
            loadedSlideshow = slideshowName;
            currentSlide = 0;
            slideTimer = ofGetElapsedTimef();
            }
            }
        // turn it on again for drawing
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
    if (isOn) {
        // save actual GL coordinates
        ofPushMatrix();

        // find transformation matrix
        findHomography(src, dst, matrix);


        //finally lets multiply our matrix
        //wooooo hoooo!
        glMultMatrixf(matrix);


        // -- NOW LETS DRAW!!!!!!  -----

        // if a solid color content or color transition is set it draws it

        // solid colors ----------------------------------------------------------------
        if (colorBg) {
            ofFill();
            ofEnableAlphaBlending();
            // if we have two colors it draws with calculated transition color
            if (transBg) {
                ofSetColor(transColor.r * 255, transColor.g * 255, transColor.b * 255, transColor.a * 255);
            }
            // this in case of only one color set
            else {
                ofSetColor(bgColor.r * 255, bgColor.g * 255, bgColor.b * 255, bgColor.a * 255);
            }
            ofRect(0, 0, ofGetWidth(), ofGetHeight());
            ofDisableAlphaBlending();
            ofNoFill();
        }

        // video ----------------------------------------------------------------------
        //if a video content is chosen it draws it
        if (videoBg) {
        ofEnableAlphaBlending();
        ofSetColor(videoColorize.r * 255, videoColorize.g * 255, videoColorize.b * 255, videoColorize.a * 255);
        if (!videoLoop) {
            // in no-looping mode it stops drawing video frame when video reaches the end
            // using 'getIsMovieDone()' because there are problems with getting head position under GStream
            if (!video.getIsMovieDone()) {
                if (videoGreenscreen) {videoTex.draw(0,0,videoWidth*videoMultX, videoHeight*videoMultY);}
                else {video.draw(0,0,videoWidth*videoMultX, videoHeight*videoMultY);}
                }
            }
        else {
            if (videoGreenscreen) {videoTex.draw(0,0,videoWidth*videoMultX, videoHeight*videoMultY);}
            else {video.draw(0,0,videoWidth*videoMultX, videoHeight*videoMultY);}
        }
        ofDisableAlphaBlending();
        }


        // camera ------------------------------------------------------------------------------
	    // camera stuff
	    if (camBg) {
        ofEnableAlphaBlending();
	    ofSetColor(camColorize.r * 255, camColorize.g * 255, camColorize.b * 255, camColorize.a * 255);
	    if (camGreenscreen) { camAlphaTexture.draw(0,0,camWidth*camMultX,camHeight*camMultY); }
	    else { camTexture.draw(0,0,camWidth*camMultX,camHeight*camMultY); }
	    ofDisableAlphaBlending();
	    }


        // draws slideshows
	    if (slideshowBg) {
        if (slides.size() > 0) {
            // if we reached the end of slides vector, it loops back to first slide
            if (currentSlide >= slides.size()) {
                currentSlide = 0;
                }
            slide = slides[currentSlide];
            ofEnableAlphaBlending();
            // color is set according to still img colorization combo
            ofSetColor(imgColorize.r * 255, imgColorize.g * 255, imgColorize.b * 255, imgColorize.a * 255);
            // default is drawing image with its size unchanged, so we set mult factors = 1.0
            float multX = 1.0;
            float multY = 1.0;
            if (slideFit) {
                float fitX = ofGetWidth()/slide.getWidth();
                float fitY = ofGetHeight()/slide.getHeight();
                if (slideKeepAspect) {
                    // we calculate the factor for fitting the image in quad respecting img aspect ratio
                    if (fitX >= fitY) {
                        multX = fitY;
                        multY = fitY;
                    }
                    else {
                        multX = fitX;
                        multY = fitX;
                    }
                }
                // this is for stretching image to whole quad size
                else {
                    multX = fitX;
                    multY = fitY;
                }
            }
            // at last we draw the image with appropriate size multiplier
            slide.draw(0,0,slide.getWidth()*multX, slide.getHeight()*multY);
            ofDisableAlphaBlending();
            // if slide showing time has elapsed it switches to next slide
            if (ofGetElapsedTimef() > slideTimer+slideshowSpeed ) {
                currentSlide += 1;
                slideTimer = ofGetElapsedTimef();
            }
        }
	    }


        //if an image content is chosen it draws it (maybe use it as mask as well?)
        if (imgBg) {
        ofEnableAlphaBlending();
        ofSetColor(imgColorize.r * 255, imgColorize.g * 255, imgColorize.b * 255, imgColorize.a * 255);
        img.draw(0,0,img.getWidth()*imgMultX, img.getHeight()*imgMultY);
        ofDisableAlphaBlending();
        }


        // TEMP STUFF - our particles
        /*
        ofEnableAlphaBlending();
        ofSetColor(255, 120, 0, 130);
        ofFill();
        for(int i = 0; i < 40; i++)balls[i].draw();
        ofDisableAlphaBlending();
        */

        //lets draw a bounding box if we are in setup mode
        ofNoFill();
        if (isSetup)
        {
            ofSetColor(borderColor);
            ofRect(0, 0, ofGetWidth(), ofGetHeight());
            // draws helper grid on active quad
            if (borderColor == 0xFFFFFF) {
                ofSetColor(0x444444);
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


        // writes quad number with a dropback shadow in the middle of quad - KEEP IT AT LAST POSITION IN draw()
        if (isSetup) {
            ofSetColor(0x000000);
            ttf.drawString("quad n. "+ofToString(quadNumber), ofGetWidth()/2, ofGetHeight()/2);
            ofSetColor(0xFFFFFF);
            ttf.drawString("quad n. "+ofToString(quadNumber), (ofGetWidth()/2)-4, (ofGetHeight()/2)-4);
        }


        // restore previous coordinates
        ofPopMatrix();
    }
    }



    void quad::gaussian_elimination(float *input, int n)
    {
        // ported to c from pseudocode in
        // http://en.wikipedia.org/wiki/Gaussian_elimination

        float * A = input;
        int i = 0;
        int j = 0;
        int m = n-1;
        while (i < m && j < n)
        {
            // Find pivot in column j, starting in row i:
            int maxi = i;
            for(int k = i+1; k<m; k++)
            {
                if(fabs(A[k*n+j]) > fabs(A[maxi*n+j]))
                {
                    maxi = k;
                }
            }
            if (A[maxi*n+j] != 0)
            {
                //swap rows i and maxi, but do not change the value of i
                if(i!=maxi)
                    for(int k=0; k<n; k++)
                    {
                        float aux = A[i*n+k];
                        A[i*n+k]=A[maxi*n+k];
                        A[maxi*n+k]=aux;
                    }
                //Now A[i,j] will contain the old value of A[maxi,j].
                //divide each entry in row i by A[i,j]
                float A_ij=A[i*n+j];
                for(int k=0; k<n; k++)
                {
                    A[i*n+k]/=A_ij;
                }
                //Now A[i,j] will have the value 1.
                for(int u = i+1; u< m; u++)
                {
                    //subtract A[u,j] * row i from row u
                    float A_uj = A[u*n+j];
                    for(int k=0; k<n; k++)
                    {
                        A[u*n+k]-=A_uj*A[i*n+k];
                    }
                    //Now A[u,j] will be 0, since A[u,j] - A[i,j] * A[u,j] = A[u,j] - 1 * A[u,j] = 0.
                }

                i++;
            }
            j++;
        }

        //back substitution
        for(int i=m-2; i>=0; i--)
        {
            for(int j=i+1; j<n-1; j++)
            {
                A[i*n+m]-=A[i*n+j]*A[j*n+m];
                //A[i*n+j]=0;
            }
        }
    }

    void quad::findHomography(ofPoint src[4], ofPoint dst[4], float homography[16])
    {

        // create the equation system to be solved
        //
        // from: Multiple View Geometry in Computer Vision 2ed
        //       Hartley R. and Zisserman A.
        //
        // x' = xH
        // where H is the homography: a 3 by 3 matrix
        // that transformed to inhomogeneous coordinates for each point
        // gives the following equations for each point:
        //
        // x' * (h31*x + h32*y + h33) = h11*x + h12*y + h13
        // y' * (h31*x + h32*y + h33) = h21*x + h22*y + h23
        //
        // as the homography is scale independent we can let h33 be 1 (indeed any of the terms)
        // so for 4 points we have 8 equations for 8 terms to solve: h11 - h32
        // after ordering the terms it gives the following matrix
        // that can be solved with gaussian elimination:

        float P[8][9]=
        {
            {-src[0].x, -src[0].y, -1,   0,   0,  0, src[0].x*dst[0].x, src[0].y*dst[0].x, -dst[0].x }, // h11
            {  0,   0,  0, -src[0].x, -src[0].y, -1, src[0].x*dst[0].y, src[0].y*dst[0].y, -dst[0].y }, // h12

            {-src[1].x, -src[1].y, -1,   0,   0,  0, src[1].x*dst[1].x, src[1].y*dst[1].x, -dst[1].x }, // h13
            {  0,   0,  0, -src[1].x, -src[1].y, -1, src[1].x*dst[1].y, src[1].y*dst[1].y, -dst[1].y }, // h21

            {-src[2].x, -src[2].y, -1,   0,   0,  0, src[2].x*dst[2].x, src[2].y*dst[2].x, -dst[2].x }, // h22
            {  0,   0,  0, -src[2].x, -src[2].y, -1, src[2].x*dst[2].y, src[2].y*dst[2].y, -dst[2].y }, // h23

            {-src[3].x, -src[3].y, -1,   0,   0,  0, src[3].x*dst[3].x, src[3].y*dst[3].x, -dst[3].x }, // h31
            {  0,   0,  0, -src[3].x, -src[3].y, -1, src[3].x*dst[3].y, src[3].y*dst[3].y, -dst[3].y }, // h32
        };

        gaussian_elimination(&P[0][0],9);

        // gaussian elimination gives the results of the equation system
        // in the last column of the original matrix.
        // opengl needs the transposed 4x4 matrix:
        float aux_H[]= { P[0][8],P[3][8],0,P[6][8], // h11  h21 0 h31
                         P[1][8],P[4][8],0,P[7][8], // h12  h22 0 h32
                         0      ,      0,0,0,       // 0    0   0 0
                         P[2][8],P[5][8],0,1
                       };      // h13  h23 0 h33

        for(int i=0; i<16; i++) homography[i] = aux_H[i];
    }



