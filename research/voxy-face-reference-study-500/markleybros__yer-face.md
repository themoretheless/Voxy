# Facial rendering source scan: markleybros/yer-face

Repository: https://github.com/markleybros/yer-face

Pinned source revision: e3968fdb3b0c3864325716fca69cc37b38be5bad

Discovery topic: facial animation

Description: YerFace! A stupid facial performance capture engine for cartoon animation.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"face": ["data/dlib-models/mmod_human_face_detector.dat", "data/dlib-models/shape_predictor_68_face_landmarks.dat", "data/yer-face-config.json", "doc/images/yer-face-logo-weboptimized.png", "doc/images/yer-face-logo.jpg", "doc/images/yer-face-logo.png", "doc/images/yer-face-logo.svg", "doc/images/yer-face.png", "src/FaceDetector.cpp", "src/FaceDetector.hpp", "src/FaceMapper.cpp", "src/FaceMapper.hpp", "src/FaceTracker.cpp", "src/FaceTracker.hpp", "src/yer-face.cpp"]}

## Source file: src/yer-face.cpp

{
  "path": "src/yer-face.cpp",
  "blob_sha": "0c9a5fd409f097b202cdbd4486788f8f2d0ef5e0",
  "sha256": "b7577ea8d73fbc18ba332deeff9ce182e5343a3ff883e2427385072acbfc46af",
  "lines": 623,
  "bytes": 24235,
  "selection_score": 15,
  "keyword_line_counts": {
    "gum": 6,
    "rig": 5,
    "face": 60
  },
  "url": "https://github.com/markleybros/yer-face/blob/e3968fdb3b0c3864325716fca69cc37b38be5bad/src/yer-face.cpp",
  "function_signature_candidates": [
    {
      "line": 109,
      "text": "int yerface(int argc, char *argv[]);"
    },
    {
      "line": 110,
      "text": "void videoCaptureInitializer(WorkerPoolWorker *worker, void *ptr);"
    },
    {
      "line": 111,
      "text": "bool videoCaptureHandler(WorkerPoolWorker *worker);"
    },
    {
      "line": 112,
      "text": "void videoCaptureDeinitializer(WorkerPoolWorker *worker, void *ptr);"
    },
    {
      "line": 113,
      "text": "void parseConfigFile(void);"
    },
    {
      "line": 114,
      "text": "void handleFrameStatusChange(void *userdata, WorkingFrameStatus newStatus, FrameTimestamps frameTimestamps);"
    },
    {
      "line": 115,
      "text": "void handleFrameServerDrainedEvent(void *userdata);"
    },
    {
      "line": 116,
      "text": "void renderPreviewHUD(Mat previewFrame, FrameNumber frameNumber, int density, bool mirrorMode);"
    },
    {
      "line": 117,
      "text": "bool fileExists(string filePath);"
    },
    {
      "line": 119,
      "text": "int main(int argc, char *argv[]) {"
    },
    {
      "line": 128,
      "text": "int yerface(int argc, char *argv[]) {"
    },
    {
      "line": 484,
      "text": "void videoCaptureInitializer(WorkerPoolWorker *worker, void *ptr) {"
    },
    {
      "line": 489,
      "text": "bool videoCaptureHandler(WorkerPoolWorker *worker) {"
    },
    {
      "line": 528,
      "text": "void videoCaptureDeinitializer(WorkerPoolWorker *worker, void *ptr) {"
    },
    {
      "line": 548,
      "text": "void parseConfigFile(void) {"
    },
    {
      "line": 569,
      "text": "void handleFrameStatusChange(void *userdata, WorkingFrameStatus newStatus, FrameTimestamps frameTimestamps) {"
    },
    {
      "line": 594,
      "text": "void handleFrameServerDrainedEvent(void *userdata) {"
    },
    {
      "line": 600,
      "text": "void renderPreviewHUD(Mat previewFrame, FrameNumber frameNumber, int density, bool mirrorMode) {"
    },
    {
      "line": 617,
      "text": "bool fileExists(string filePath) {"
    }
  ]
}

```text


#include "Logger.hpp"
#include "Status.hpp"
#include "SDLDriver.hpp"
#include "FFmpegDriver.hpp"
#include "FrameServer.hpp"
#include "FaceDetector.hpp"
#include "FaceTracker.hpp"
#include "FaceMapper.hpp"
#include "Metrics.hpp"
#include "Utilities.hpp"
#include "OutputDriver.hpp"
#include "SphinxDriver.hpp"
#include "EventLogger.hpp"
#include "PreviewHUD.hpp"
#include "WorkerPool.hpp"

#include <iostream>
#include <sstream>
#include <cstdio>
#include <cstdlib>

#include <sys/types.h>
#include <sys/stat.h>

using namespace std;
using namespace cv;
using namespace YerFace;

string configFile;

string inVideo;
string inVideoFormat;
string inVideoSize;
string inVideoRate;
string inVideoCodec;

string inAudio;
string inAudioFormat;
string inAudioChannels;
string inAudioRate;
string inAudioCodec;

string inAudioChannelMap;

string inEventData;
double inEventDataStartSeconds = 0.0;

string outEventData;
string outVideo;
string outLogFile;
string outLogColors;
string outLogColorsString = "";

string previewMirror;
bool previewMirrorBool = false;
bool lowLatency = false;
bool headless = false;
bool previewAudio = false;
bool tryAudioInVideo = false;
bool openInputAudio = false;
bool stdinPipeUsed = false;

int verbosity = 0, logSeverityFilter = LOG_SEVERITY_FILTERDEFAULT;

json config = NULL;

SDLWindowRenderer sdlWindowRenderer;
bool windowInitializationFailed;
WorkerPool *videoCaptureWorkerPool;

Status *status = NULL;
Logger *logger = NULL;
SDLDriver *sdlDriver = NULL;
FFmpegDriver *ffmpegDriver = NULL;
FrameServer *frameServer = NULL;
FaceDetector *faceDetector = NULL;
FaceTracker *faceTracker = NULL;
FaceMapper *faceMapper = NULL;
Metrics *metrics = NULL;
Metrics *previewMetrics = NULL;
OutputDriver *outputDriver = NULL;
SphinxDriver *sphinxDriver = NULL;
EventLogger *eventLogger = NULL;
PreviewHUD *previewHUD = NULL;

//VARIABLES PROTECTED BY frameSizeMutex
Size frameSize;
bool frameSizeValid = false;
SDL_mutex *frameSizeMutex;
//END VARIABLES PROTECTED BY frameSizeMutex

//VARIABLES PROTECTED BY previewDisplayMutex
list<FrameNumber> previewDisplayFrameNumbers;
SDL_mutex *previewDisplayMutex;
SDL_cond *previewDisplayCond;
//END VARIABLES PROTECTED BY previewDisplayMutex

//VARIABLES PROTECTED BY frameServerDrainedMutex
bool frameServerDrained = false;
SDL_mutex *frameServerDrainedMutex;
//END VARIABLES PROTECTED BY frameServerDrainedMutex

//VARIABLES PROTECTED BY frameMetricsMutex
unordered_map<FrameNumber, MetricsTick> frameMetricsTicks;
SDL_mutex *frameMetricsMutex;
//END VARIABLES PROTECTED BY frameMetricsMutex

int yerface(int argc, char *argv[]);
void videoCaptureInitializer(WorkerPoolWorker *worker, void *ptr);
bool videoCaptureHandler(WorkerPoolWorker *worker);
void videoCaptureDeinitializer(WorkerPoolWorker *worker, void *ptr);
void parseConfigFile(void);
void handleFrameStatusChange(void *userdata, WorkingFrameStatus newStatus, FrameTimestamps frameTimestamps);
void handleFrameServerDrainedEvent(void *userdata);
void renderPreviewHUD(Mat previewFrame, FrameNumber frameNumber, int density, bool mirrorMode);
bool fileExists(string filePath);

int main(int argc, char *argv[]) {
	try {
		return yerface(argc, argv);
	} catch(exception &e) {
		Logger::slog("Main", LOG_SEVERITY_CRIT, "Uncaught exception in parent thread: %s", e.what());
	}
	return 1;
}

int yerface(int argc, char *argv[]) {
	//Command line options. NOTE: Remember to update the documentation when making changes here!
	CommandLineParser parser(argc, argv,
		"{help h usage ?||Display command line usage documentation.}"
		"{configFile||Required configuration file. (Indicate the full or relative path to your 'yer-face-config.json' file. Omit to search common locations.)}"
		"{lowLatency||If true, will tweak behavior across the system to minimize latency. (Don't use this if the input is pre-recorded!)}"
		"{inVideo||Video file, URL, or device to open. (Or '-' for STDIN.)}"
		"{inVideoFormat||Tell libav to use a specific format to interpret the inVideo. Leave blank for auto-detection.}"
		"{inVideoSize||Tell libav to attempt a specific resolution when interpreting inVideo. Leave blank for auto-detection.}"
		"{inVideoRate||Tell libav to attempt a specific framerate when interpreting inVideo. Leave blank for auto-detection.}"
		"{inVideoCodec||Tell libav to attempt a specific codec when interpreting inVideo. Leave blank for auto-detection.}"
		"{inAudio||Audio file, URL, or device to open. Alternatively: '' (blank string, the default) we will try to read the audio from inVideo. '-' we will try to read the audio from STDIN. 'ignore' we will ignore all audio from all sources.}"
		"{inAudioFormat||Tell libav to use a specific format to interpret the inAudio. Leave blank for auto-detection.}"
		"{inAudioChannels||Tell libav to attempt a specific number of channels when interpreting inAudio. Leave blank for auto-detection.}"
		"{inAudioRate||Tell libav to attempt a specific sample rate when interpreting inAudio. Leave blank for auto-detection.}"
		"{inAudioCodec||Tell libav to attempt a specific codec when interpreting inAudio. Leave blank for auto-detection.}"
		"{inAudioChannelMap||Alter the input audio channel mapping. Set to \"left\" to interpret only the left channel, \"right\" to interpret only the right channel, and leave blank for the default.}"
		"{inEventData||Input event data / replay file. (Previously generated outEventData, for re-processing recorded sessions.)}"
		"{inEventDataStartSeconds|0.0|Offset for input event data / replay file timestamps. (Useful if the capture session was trimmed.)}"
		"{outEventData||Output event data / replay file. (Includes performance capture data.)}"
		"{outVideo||Output file for captured video and audio. Together with the \"outEventData\" file, this can be used to re-run a previous capture session.}"
		"{outLogFile||If specified, log messages will be written to this file. If \"-\" or not specified, log messages will be written to STDERR.}"
		"{outLogColors||If true, log colorization will be forced on. If false, log colorization will be forced off. If \"auto\" or not specified, log colorization will auto-detect.}"
		"{previewAudio||If true, will preview processed audio out the computer's sound device.}"
		"{previewMirror||If true, mirror mode (horizontal reflection) of the preview will be forced on. If false, mirror mode will be forced off. If \"auto\" or not specified, mirror mode will be enabled for lowLatency mode and disabled otherwise.}"
		"{headless||If set, all video display and audio playback is disabled. Intended to be suitable for jobs running in the terminal.}"
		"{version||Emit the version string to STDOUT and exit.}"
		"{verbosity verbose v||Adjust the log level filter. Indicate a positive number to increase the verbosity, a negative number to decrease the verbosity, or specify with no integer to increase the verbosity to a moderate degree.)}"
		);
	parser.about("YerFace! A stupid facial performance capture engine for cartoon animation. [" YERFACE_VERSION "]");
	if(argc <= 1) {
		parser.printMessage();
		return 0;
	}
	if(parser.has("version") && parser.get<bool>("version")) {
		fprintf(stdout, "%s\n", YERFACE_VERSION);
		return 0;
	}
	if(parser.has("help") && parser.get<bool>("help")) {
		parser.printMessage();
		return 1;
	}
	if(parser.has("verbosity")) {
		string verbosityString = parser.get<string>("verbosity");
		if(verbosityString.length() == 0 || verbosityString == "true") {
			verbosity = 2;
		} else {
			verbosity = parser.get<int>("verbosity");
		}
		logSeverityFilter = (int)LOG_SEVERITY_FILTERDEFAULT + verbosity;
		if(logSeverityFilter < (int)LOG_SEVERITY_MIN) {
			logSeverityFilter = LOG_SEVERITY_MIN;
		} else if(logSeverityFilter > (int)LOG_SEVERITY_MAX) {
			logSeverityFilter = LOG_SEVERITY_MAX;
		}
		Logger::setLoggingFilter((LogMessageSeverity)logSeverityFilter);
	}
	configFile = parser.get<string>("configFile");
	inVideo = parser.get<string>("inVideo");
	if(inVideo.length() == 0) {
		throw invalid_argument("--inVideo is a required argument, but is blank or not specified!");
	}
	if(inVideo == "-") {
		inVideo = "pipe:0";
		stdinPipeUsed = true;
	}
	inVideoFormat = parser.get<string>("inVideoFormat");
	inVideoSize = parser.get<string>("inVideoSize");
	inVideoRate = parser.get<string>("inVideoRate");
	inVideoCodec = parser.get<string>("inVideoCodec");
	inAudio = parser.get<string>("inAudio");
	if(inAudio == "-") {
		inAudio = "pipe:0";
		if(stdinPipeUsed) {
			throw invalid_argument("Can't use STDIN for both --inVideo and --inAudio! If you need to read Video and Audio from the same pipe, use a tool to multiplex them first.");
		}
		stdinPipeUsed = true;
	}
	if(inAudio.length() > 0) {
		if(inAudio == "ignore") {
			openInputAudio = false;
			tryAudioInVideo = false;
		} else {
			openInputAudio = true;
			tryAudioInVideo = false;
		}
	} else {
		openInputAudio = false;
		tryAudioInVideo = true;
	}
	inAudioFormat = parser.get<string>("inAudioFormat");
	inAudioChannels = parser.get<string>("inAudioChannels");
	inAudioRate = parser.get<string>("inAudioRate");
	inAudioCodec = parser.get<string>("inAudioCodec");
	inAudioChannelMap = parser.get<string>("inAudioChannelMap");
	inEventData = parser.get<string>("inEventData");
	inEventDataStartSeconds = parser.get<double>("inEventDataStartSeconds");
	outEventData = parser.get<string>("outEventData");
	outVideo = parser.get<string>("outVideo");
	outLogFile = parser.get<string>("outLogFile");
	outLogColors = parser.get<string>("outLogColors");
	lowLatency = parser.has("lowLatency") && parser.get<bool>("lowLatency");
	headless = parser.has("headless") && parser.get<bool>("headless");
	previewAudio = parser.has("previewAudio") && parser.get<bool>("previewAudio");
	previewMirror = parser.get<string>("previewMirror");

	if(!parser.check()) {
		parser.printErrors();
		parser.printMessage();
		return 1;
	}

	sdlWindowRenderer.window = NULL;
	sdlWindowRenderer.renderer = NULL;
	windowInitializationFailed = false;

	if(outLogFile.length() == 0 || outLogFile == "-") {
		outLogFile = "-";
		Logger::setLoggingTarget(stderr);
	} else {
		if(fileExists(outLogFile)) {
			throw invalid_argument("Refusing to overwrite outLogFile. Specified file already exists!");
		}
		Logger::setLoggingTarget(outLogFile);
	}
	if(outLogColors.length() == 0 || outLogColors == "auto") {
		Logger::setLoggingColorMode(LOG_COLORS_AUTO);
		outLogColorsString = "AUTO";
	} else {
		if(parser.get<bool>("outLogColors")) {
			Logger::setLoggingColorMode(LOG_COLORS_ON);
			outLogColorsString = "ON";
		} else {
			Logger::setLoggingColorMode(LOG_COLORS_OFF);
			outLogColorsString = "OFF";
		}
	}

	if(previewMirror.length() == 0 || previewMirror == "auto") {
		previewMirrorBool = lowLatency;
	} else {
		previewMirrorBool = parser.get<bool>("previewMirror");
	}

	logger = new Logger("YerFace");
	logger->notice("Starting up...");
	logger->info("Log output is being sent to: %s", outLogFile == "-" ? "STDERR" : outLogFile.c_str());
	logger->info("Log filter is set to: %s", Logger::getSeverityString((LogMessageSeverity)logSeverityFilter).c_str());
	logger->info("Log colorization mode is: %s", outLogColorsString.c_str());

	//Create locks and conditions.
	if((frameSizeMutex = SDL_CreateMutex()) == NULL) {
		throw runtime_error("Failed creating mutex!");
	}
	if((previewDisplayMutex = SDL_CreateMutex()) == NULL) {
		throw runtime_error("Failed creating mutex!");
	}
	if((previewDisplayCond = SDL_CreateCond()) == NULL) {
		throw runtime_error("Failed creating condition!");
	}
	if((frameServerDrainedMutex = SDL_CreateMutex()) == NULL) {
		throw runtime_error("Failed creating mutex!");
	}
	if((frameMetricsMutex = SDL_CreateMutex()) == NULL) {
		throw runtime_error("Failed creating mutex!");
	}


	//Initialize configuration.
	parseConfigFile();

	//Instantiate our classes.
	status = new Status(lowLatency);
	metrics = new Metrics(config, "YerFace", true);
	previewMetrics = new Metrics(config, "YerFace[Preview/Event Loop]", false);
	frameServer = new FrameServer(config, status, lowLatency);
	previewHUD = new PreviewHUD(config, status, frameServer, previewMirrorBool);
	ffmpegDriver = new FFmpegDriver(status, frameServer, lowLatency, false);
	ffmpegDriver->openInputMedia(inVideo, AVMEDIA_TYPE_VIDEO, inVideoFormat, inVideoSize, "", inVideoRate, inVideoCodec, inAudioChannelMap, tryAudioInVideo);
	if(openInputAudio) {
		ffmpegDriver->openInputMedia(inAudio, AVMEDIA_TYPE_AUDIO, inAudioFormat, "", inAudioChannels, inAudioRate, inAudioCodec, inAudioChannelMap, true);
	}
	if(outVideo.length() > 0) {
		if(fileExists(outVideo)) {
			throw invalid_argument("Refusing to overwrite outVideo. Specified file already exists!");
		}
		ffmpegDriver->openOutputMedia(outVideo);
	}
	sdlDriver = new SDLDriver(config, status, frameServer, ffmpegDriver, headless, previewAudio && ffmpegDriver->getIsAudioInputPresent());
	faceDetector = new FaceDetector(config, status, frameServer);
	faceTracker = new FaceTracker(config, status, sdlDriver, frameServer, faceDetector);
	faceMapper = new FaceMapper(config, status, frameServer, faceTracker, previewHUD);
	if(outEventData.length() > 0 && fileExists(outEventData)) {
		throw invalid_argument("Refusing to overwrite outEventData. Specified file already exists!");
	}
	outputDriver = new OutputDriver(config, outEventData, status, frameServer, faceTracker, sdlDriver);
	if(ffmpegDriver->getIsAudioInputPresent()) {
		sphinxDriver = new SphinxDriver(config, status, frameServer, ffmpegDriver, sdlDriver, outputDriver, previewHUD, lowLatency);
	}
	eventLogger = new EventLogger(config, inEventData, inEventDataStartSeconds, status, outputDriver, frameServer);

	outputDriver->setEventLogger(eventLogger);

	//Register preview renderers.
	previewHUD->registerPreviewHUDRenderer(renderPreviewHUD);

	//Hook into the frame lifecycle.
	FrameServerDrainedEventCallback frameServerDrainedCallback;
	frameServerDrainedCallback.userdata = NULL;
	frameServerDrainedCallback.callback = handleFrameServerDrainedEvent;
	frameServer->onFrameServerDrainedEvent(frameServerDrainedCallback);

	FrameStatusChangeEventCallback frameStatusChangeCallback;
	frameStatusChangeCallback.userdata = NULL;
	frameStatusChangeCallback.callback = handleFrameStatusChange;
	frameStatusChangeCallback.newStatus = FRAME_STATUS_NEW;
	frameServer->onFrameStatusChangeEvent(frameStatusChangeCallback);
	if(!headless) {
		frameStatusChangeCallback.newStatus = FRAME_STATUS_PREVIEW_DISPLAY;
		frameServer->onFrameStatusChangeEvent(frameStatusChangeCallback);
		frameServer->registerFrameStatusCheckpoint(FRAME_STATUS_PREVIEW_DISPLAY, "main.PreviewDisplayed");
	}
	frameStatusChangeCallback.newStatus = FRAME_STATUS_GONE;
	frameServer->onFrameStatusChangeEvent(frameStatusChangeCallback);

	//Create worker thread.
	WorkerPoolParameters workerPoolParameters;
	workerPoolParameters.name = "Main.VideoCapture";
	workerPoolParameters.numWorkers = 1;
	workerPoolParameters.numWorkersPerCPU = 0.0;
	workerPoolParameters.initializer = videoCaptureInitializer;
	workerPoolParameters.deinitializer = videoCaptureDeinitializer;
	workerPoolParameters.usrPtr = NULL;
	workerPoolParameters.handler = videoCaptureHandler;
	videoCaptureWorkerPool = new WorkerPool(config, status, frameServer, workerPoolParameters);

	//Launch event / rendering loop.
	bool myDrained = false;
	std::list<FrameNumber> previewFrames;
	previewFrames.clear();
	FrameNumber previewTargetFrameNumber = -1;
	while((status->getIsRunning() || !myDrained) && !status->getEmergency()) {
		MetricsTick tick = previewMetrics->startClock();

		// Window initialization.
		if(!headless && sdlWindowRenderer.window == NULL && !windowInitializationFailed) {
			YerFace_MutexLock(frameSizeMutex);
			if(!frameSizeValid) {
				YerFace_MutexUnlock(frameSizeMutex);
				continue;
			}
			try {
				sdlWindowRenderer = sdlDriver->createPreviewWindow(frameSize.width, frameSize.height, "YerFace! Preview Window");
			} catch(exception &e) {
				windowInitializationFailed = true;
				logger->err("Uh oh, failed to create a preview window! Got exception: %s", e.what());
				logger->notice("Continuing despite the lack of a preview window.");
			}
			YerFace_MutexUnlock(frameSizeMutex);
		}

		//Preview frame display.
		if(!headless) {
			while(previewFrames.size() > 0) {
				if(previewTargetFrameNumber != -1) {
					// We had a previous "target" preview frame, we should release it from the pipeline.
					frameServer->setWorkingFrameStatusCheckpoint(previewTargetFrameNumber, FRAME_STATUS_PREVIEW_DISPLAY, "main.PreviewDisplayed");
				}
				previewTargetFrameNumber = previewFrames.back();
				previewFrames.pop_back();
			}

			if(previewTargetFrameNumber != -1) {
				WorkingFrame *previewFrame = frameServer->getWorkingFrame(previewTargetFrameNumber);
				YerFace_MutexLock(previewFrame->previewFrameMutex);
				Mat previewFrameCopy = previewFrame->previewFrame.clone();
				YerFace_MutexUnlock(previewFrame->previewFrameMutex);
				previewHUD->doRenderPreviewHUD(previewFrameCopy, previewTargetFrameNumber);
				sdlDriver->doRenderPreviewFrame(previewFrameCopy);
			}
		}

		// If we're shutting down, don't hang on to the previous frame.
		if(!status->getIsRunning()) {
			if(previewTargetFrameNumber != -1) {
				frameServer->setWorkingFrameStatusCheckpoint(previewTargetFrameNumber, FRAME_STATUS_PREVIEW_DISPLAY, "main.PreviewDisplayed");
				previewTargetFrameNumber = -1;
			}
		}

		//SDL bookkeeping.
		sdlDriver->doHandleEvents();

		previewMetrics->endClock(tick);

		//Loop timing is semi-regulated by condition signalling.
		YerFace_MutexLock(previewDisplayMutex);
		if(status->getIsRunning() && previewDisplayFrameNumbers.size() == 0) {
			// Condition timeout needs to be very short because our our responsiveness to input events depends on it.
			int result = SDL_CondWaitTimeout(previewDisplayCond, previewDisplayMutex, 1);
			if(result < 0) {
				throw runtime_error("CondWaitTimeout() failed!");
			}
		}
		//Take note of any preview frames waiting to be displayed so they can be handled.
		while(previewDisplayFrameNumbers.size() > 0) {
			previewFrames.push_front(previewDisplayFrameNumbers.back());
			previewDisplayFrameNumbers.pop_back();
		}
		YerFace_MutexUnlock(previewDisplayMutex);

		YerFace_MutexLock(frameServerDrainedMutex);
		myDrained = frameServerDrained;
		YerFace_MutexUnlock(frameServerDrainedMutex);
	}

	//Join worker thread.
	YerFace_CarefullyDelete(logger, status, videoCaptureWorkerPool);

	//Cleanup.
	YerFace_CarefullyDelete(logger, status, eventLogger);
	if(sphinxDriver != NULL) {
		delete sphinxDriver;
	}
	YerFace_CarefullyDelete(logger, status, outputDriver);
	YerFace_CarefullyDelete(logger, status, faceMapper);
	YerFace_CarefullyDelete(logger, status, faceTracker);
	YerFace_CarefullyDelete(logger, status, faceDetector);
	YerFace_CarefullyDelete(logger, status, previewHUD);
	YerFace_CarefullyDelete(logger, status, frameServer);
	YerFace_CarefullyDelete(logger, status, ffmpegDriver);
	YerFace_CarefullyDelete(logger, status, sdlDriver);
	YerFace_CarefullyDelete(logger, status, previewMetrics);
	YerFace_CarefullyDelete(logger, status, metrics);
	YerFace_CarefullyDelete_NoStatus(logger, status);
	try {
		logger->notice("Goodbye!");
		delete logger;
	} catch(exception &e) {
		Logger::slog("YerFace", LOG_SEVERITY_EMERG, "Logger Destructor exception: %s", e.what());
	}

	// If we previously opened a file as a logging target,
	// setting a new logging target will force the old file to be closed.
	// Otherwise this line will have no effect.
	Logger::setLoggingTarget(stderr);

	SDL_DestroyMutex(frameSizeMutex);
	SDL_DestroyMutex(previewDisplayMutex);
	SDL_DestroyCond(previewDisplayCond);
	SDL_DestroyMutex(frameServerDrainedMutex);
	SDL_DestroyMutex(frameMetricsMutex);

	return 0;
}

void videoCaptureInitializer(WorkerPoolWorker *worker, void *ptr) {
	ffmpegDriver->setVideoCaptureWorkerPool(worker->pool);
	ffmpegDriver->rollWorkerThreads();
}

bool videoCaptureHandler(WorkerPoolWorker *worker) {
	VideoFrame videoFrame;
	bool didWork = false;
	static bool didSetFrameSizeValid = false;

	int demuxerRunning = ffmpegDriver->pollForNextVideoFrame(&videoFrame);
	if(videoFrame.valid) {
		if(!didSetFrameSizeValid) {
			YerFace_MutexLock(frameSizeMutex);
			if(!frameSizeValid) {
				frameSize = videoFrame.frameCV.size();
				frameSizeValid = true;
				didSetFrameSizeValid = true;
			}
			YerFace_MutexUnlock(frameSizeMutex);
		}
		frameServer->insertNewFrame(&videoFrame);
		ffmpegDriver->releaseVideoFrame(videoFrame);
		didWork = true;
	}

	if(!videoFrame.valid && !demuxerRunning) {
		logger->info("FFmpeg Demuxer thread finished.");
		status->setIsRunning(false);
	}

	if(!status->getIsRunning()) {
		logger->debug4("Issuing videoCaptureWorkerPool->stopWorkerNow()");
		worker->pool->stopWorkerNow();
		didWork = true;
	}

	if(!didWork) {
		logger->debug4("Main Video Capture thread woke up, found no work to do, and went back to sleep.");
	}

	return didWork;
}

void videoCaptureDeinitializer(WorkerPoolWorker *worker, void *ptr) {
	sdlDriver->stopAudioDriverNow();

	if(status->getEmergency()) {
		return;
	}

	frameServer->setDraining();

	bool myDrained = false;
	do {
		SDL_Delay(100);
		YerFace_MutexLock(frameServerDrainedMutex);
		myDrained = frameServerDrained;
		YerFace_MutexUnlock(frameServerDrainedMutex);
	} while(!myDrained);

	ffmpegDriver->stopAudioCallbacksNow();
}

void parseConfigFile(void) {
	if(configFile.length() < 1) {
		configFile = Utilities::fileValidPathOrDie("yer-face-config.json", true);
	} else {
		configFile = Utilities::fileValidPathOrDie(configFile);
	}
	try {
		logger->info("Opening and parsing config file: \"%s\"", configFile.c_str());
		std::ifstream fileStream = std::ifstream(configFile);
		if(fileStream.fail()) {
			throw invalid_argument("Specified config file failed to open.");
		}
		std::stringstream ssBuffer;
		ssBuffer << fileStream.rdbuf();
		config = json::parse(ssBuffer.str());
	} catch(exception &e) {
		logger->err("Failed to parse configuration file \"%s\". Got exception: %s", configFile.c_str(), e.what());
		throw;
	}
}

void handleFrameStatusChange(void *userdata, WorkingFrameStatus newStatus, FrameTimestamps frameTimestamps) {
	FrameNumber frameNumber = frameTimestamps.frameNumber;
	switch(newStatus) {
		default:
			throw logic_error("Handler passed unsupported frame status change event!");
		case FRAME_STATUS_NEW:
			YerFace_MutexLock(frameMetricsMutex);
			frameMetricsTicks[frameNumber] = metrics->startClock();
			YerFace_MutexUnlock(frameMetricsMutex);
			break;
		case FRAME_STATUS_PREVIEW_DISPLAY:
			YerFace_MutexLock(previewDisplayMutex);
			previewDisplayFrameNumbers.push_front(frameNumber);
			SDL_CondSignal(previewDisplayCond);
			YerFace_MutexUnlock(previewDisplayMutex);
			break;
		case FRAME_STATUS_GONE:
			YerFace_MutexLock(frameMetricsMutex);
			metrics->endClock(frameMetricsTicks[frameNumber]);
			frameMetricsTicks.erase(frameNumber);
			YerFace_MutexUnlock(frameMetricsMutex);
			break;
	}
}

void handleFrameServerDrainedEvent(void *userdata) {
	YerFace_MutexLock(frameServerDrainedMutex);
	frameServerDrained = true;
	YerFace_MutexUnlock(frameServerDrainedMutex);
}

void renderPreviewHUD(Mat previewFrame, FrameNumber frameNumber, int density, bool mirrorMode) {
	faceDetector->renderPreviewHUD(previewFrame, frameNumber, density, mirrorMode);
	faceTracker->renderPreviewHUD(previewFrame, frameNumber, density, mirrorMode);
	faceMapper->renderPreviewHUD(previewFrame, frameNumber, density, mirrorMode);
	if(sphinxDriver != NULL) {
		sphinxDriver->renderPreviewHUD(previewFrame, frameNumber, density, mirrorMode);
	}
	double fontSize = 1.25 * (previewFrame.size().height / 720.0); // FIXME - magic numbers
	int baseline;
	Size textSize = Utilities::getTextSize(metrics->getTimesString().c_str(), &baseline, fontSize);
	int lineskip = textSize.height + baseline;
	Point origin = Point(25, lineskip + 25);
	Utilities::drawText(previewFrame, metrics->getTimesString().c_str(), origin, Scalar(200,200,200), fontSize);
	origin.y += lineskip;
	Utilities::drawText(previewFrame, metrics->getFPSString().c_str(), origin, Scalar(200,200,200), fontSize);
}

bool fileExists(string filePath) {
	struct stat statbuf;
	if(stat(filePath.c_str(), &statbuf) == 0) {
		return true;
	}
	return false;
}


```

## Source file: src/FaceMapper.cpp

{
  "path": "src/FaceMapper.cpp",
  "blob_sha": "f7219e60fb4e1cb642cc89eda9b10a06bc6397d1",
  "sha256": "ea9b9092d982304b1ee537a04a347e66e479d7e22e475ae777bf550f5f581305",
  "lines": 293,
  "bytes": 10558,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 30,
    "jaw": 3,
    "gum": 4,
    "rig": 26,
    "face": 42,
    "lip": 21
  },
  "url": "https://github.com/markleybros/yer-face/blob/e3968fdb3b0c3864325716fca69cc37b38be5bad/src/FaceMapper.cpp",
  "function_signature_candidates": []
}

```text


#include "FaceMapper.hpp"
#include "Utilities.hpp"
#include "opencv2/calib3d.hpp"

#include <cstdlib>

using namespace std;
using namespace cv;

namespace YerFace {

FaceMapper::FaceMapper(json config, Status *myStatus, FrameServer *myFrameServer, FaceTracker *myFaceTracker, PreviewHUD *myPreviewHUD) {
	workerPool = NULL;
	status = myStatus;
	if(status == NULL) {
		throw invalid_argument("status cannot be NULL");
	}
	frameServer = myFrameServer;
	if(frameServer == NULL) {
		throw invalid_argument("frameServer cannot be NULL");
	}
	faceTracker = myFaceTracker;
	if(faceTracker == NULL) {
		throw invalid_argument("faceTracker cannot be NULL");
	}
	previewHUD = myPreviewHUD;
	if(previewHUD == NULL) {
		throw invalid_argument("previewHUD cannot be NULL");
	}

	logger = new Logger("FaceMapper");
	metrics = new Metrics(config, "FaceMapper", frameServer);

	markerEyelidLeftTop = new MarkerTracker(config, EyelidLeftTop, this);
	markerEyelidRightTop = new MarkerTracker(config, EyelidRightTop, this);
	markerEyelidLeftBottom = new MarkerTracker(config, EyelidLeftBottom, this);
	markerEyelidRightBottom = new MarkerTracker(config, EyelidRightBottom, this);

	markerEyebrowLeftInner = new MarkerTracker(config, EyebrowLeftInner, this);
	markerEyebrowRightInner = new MarkerTracker(config, EyebrowRightInner, this);
	markerEyebrowLeftMiddle = new MarkerTracker(config, EyebrowLeftMiddle, this);
	markerEyebrowRightMiddle = new MarkerTracker(config, EyebrowRightMiddle, this);
	markerEyebrowLeftOuter = new MarkerTracker(config, EyebrowLeftOuter, this);
	markerEyebrowRightOuter = new MarkerTracker(config, EyebrowRightOuter, this);

	markerJaw = new MarkerTracker(config, Jaw, this);

	markerLipsLeftCorner = new MarkerTracker(config, LipsLeftCorner, this);
	markerLipsRightCorner = new MarkerTracker(config, LipsRightCorner, this);

	markerLipsLeftTop = new MarkerTracker(config, LipsLeftTop, this);
	markerLipsRightTop = new MarkerTracker(config, LipsRightTop, this);

	markerLipsLeftBottom = new MarkerTracker(config, LipsLeftBottom, this);
	markerLipsRightBottom = new MarkerTracker(config, LipsRightBottom, this);
	
	trackers = {
		markerEyelidLeftTop,
		markerEyelidLeftBottom,
		markerEyelidRightTop,
		markerEyelidRightBottom,
		markerEyebrowLeftInner,
		markerEyebrowLeftMiddle,
		markerEyebrowLeftOuter,
		markerEyebrowRightInner,
		markerEyebrowRightMiddle,
		markerEyebrowRightOuter,
		markerJaw,
		markerLipsLeftCorner,
		markerLipsRightCorner,
		markerLipsLeftTop,
		markerLipsRightTop,
		markerLipsLeftBottom,
		markerLipsRightBottom
	};

	pendingFrames.clear();

	if((myMutex = SDL_CreateMutex()) == NULL) {
		throw runtime_error("Failed creating mutex!");
	}

	//Hook into the frame lifecycle.

	//We want to know when any frame has entered various statuses.
	FrameStatusChangeEventCallback frameStatusChangeCallback;
	frameStatusChangeCallback.userdata = (void *)this;
	frameStatusChangeCallback.callback = handleFrameStatusChange;
	frameStatusChangeCallback.newStatus = FRAME_STATUS_NEW;
	frameServer->onFrameStatusChangeEvent(frameStatusChangeCallback);
	frameStatusChangeCallback.newStatus = FRAME_STATUS_MAPPING;
	frameServer->onFrameStatusChangeEvent(frameStatusChangeCallback);
	frameStatusChangeCallback.newStatus = FRAME_STATUS_GONE;
	frameServer->onFrameStatusChangeEvent(frameStatusChangeCallback);

	//We also want to introduce a checkpoint so that frames cannot TRANSITION AWAY from FRAME_STATUS_MAPPING without our blessing.
	frameServer->registerFrameStatusCheckpoint(FRAME_STATUS_MAPPING, "faceMapper.ran");

	WorkerPoolParameters workerPoolParameters;
	workerPoolParameters.name = "FaceMapper";
	workerPoolParameters.numWorkers = 1; //FaceMapper (and MarkerTracker) cannot handle out-of-order frame processing.
	workerPoolParameters.numWorkersPerCPU = 0.0;
	workerPoolParameters.initializer = NULL;
	workerPoolParameters.deinitializer = NULL;
	workerPoolParameters.usrPtr = (void *)this;
	workerPoolParameters.handler = workerHandler;
	workerPool = new WorkerPool(config, status, frameServer, workerPoolParameters);

	logger->debug1("FaceMapper object constructed and ready to go!");
}

FaceMapper::~FaceMapper() noexcept(false) {
	logger->debug1("FaceMapper object destructing...");

	delete workerPool;

	YerFace_MutexLock(myMutex);
	if(pendingFrames.size() > 0) {
		logger->err("Frames are still pending! Woe is me!");
	}
	YerFace_MutexUnlock(myMutex);

	for(MarkerTracker *markerTracker : trackers) {
		if(markerTracker != NULL) {
			delete markerTracker;
		}
	}
	SDL_DestroyMutex(myMutex);
	delete metrics;
	delete logger;
}

void FaceMapper::renderPreviewHUD(Mat frame, FrameNumber frameNumber, int density, bool mirrorMode) {
	for(MarkerTracker *markerTracker : trackers) {
		markerTracker->renderPreviewHUD(frame, frameNumber, density, mirrorMode);
	}
	if(density > 0) {
		Scalar neutralColor = Scalar(255, 255, 255);
		Scalar leftColor = Scalar(255, 0, 0);
		Scalar rightColor = Scalar(0, 0, 255);
		int gridIncrement = 15; //FIXME - magic numbers
		Rect2d previewRect;
		Point2d previewCenter;
		previewHUD->createPreviewHUDRectangle(frame.size(), &previewRect, &previewCenter);
		rectangle(frame, previewRect, Scalar(20, 20, 20), FILLED);
		//// FIXME - grid should be related to real-world units if possible
		if(density > 4) {
			for(int x = (int)previewRect.x; x < (int)(previewRect.x + previewRect.width); x = x + gridIncrement) {
				cv::line(frame, Point2d(x, previewRect.y), Point2d(x, previewRect.y + previewRect.height), Scalar(75, 75, 75), 1, LINE_AA); // FIXME - proportional drawing
			}
			for(int y = (int)previewRect.y; y < (int)(previewRect.y + previewRect.height); y = y + gridIncrement) {
				cv::line(frame, Point2d(previewRect.x, y), Point2d(previewRect.x + previewRect.width, y), Scalar(75, 75, 75), 1, LINE_AA); // FIXME - proportional drawing
			}
		}
		double previewPointScale = previewRect.width / 200; // FIXME - more magic numbers?
		double mirrorFlip = 1.0;
		if(mirrorMode) {
			mirrorFlip = -1.0;
		}
		for(MarkerTracker *markerTracker : trackers) {
			MarkerPoint markerPoint = markerTracker->getMarkerPoint(frameNumber);
			if(markerPoint.set) {
				Point2d previewPoint = Point2d(
						(markerPoint.point3d.x * previewPointScale * mirrorFlip) + previewCenter.x,
						(markerPoint.point3d.y * previewPointScale) + previewCenter.y);
				bool didDraw = false;
				if(density >= 5) {
					switch(markerTracker->getMarkerType().type) {
						default:
						case Jaw:
							break;
						case EyelidLeftTop:
						case EyelidLeftBottom:
						case EyebrowLeftInner:
						case EyebrowLeftMiddle:
						case EyebrowLeftOuter:
						case LipsLeftCorner:
						case LipsLeftTop:
						case LipsLeftBottom:
							Utilities::drawX(frame, previewPoint, leftColor); // FIXME - proportional drawing
							break;
						case EyelidRightTop:
						case EyelidRightBottom:
						case EyebrowRightInner:
						case EyebrowRightMiddle:
						case EyebrowRightOuter:
						case LipsRightCorner:
						case LipsRightTop:
						case LipsRightBottom:
							Utilities::drawX(frame, previewPoint, rightColor); // FIXME - proportional drawing
							break;
					}
				}
				if(!didDraw) {
					Utilities::drawX(frame, previewPoint, neutralColor); // FIXME - proportional drawing
				}
			}
		}
	}
}

FrameServer *FaceMapper::getFrameServer(void) {
	return frameServer;
}

FaceTracker *FaceMapper::getFaceTracker(void) {
	return faceTracker;
}

void FaceMapper::handleFrameStatusChange(void *userdata, WorkingFrameStatus newStatus, FrameTimestamps frameTimestamps) {
	FrameNumber frameNumber = frameTimestamps.frameNumber;
	FaceMapper *self = (FaceMapper *)userdata;
	FaceMapperPendingFrame newFrame;
	self->logger->debug4("Handling Frame Status Change for Frame Number " YERFACE_FRAMENUMBER_FORMAT " to Status %d", frameNumber, newStatus);
	switch(newStatus) {
		default:
			throw logic_error("Handler passed unsupported frame status change event!");
		case FRAME_STATUS_NEW:
			newFrame.frameNumber = frameNumber;
			newFrame.hasEnteredMapping = false;
			newFrame.hasCompletedMapping = false;
			YerFace_MutexLock(self->myMutex);
			self->pendingFrames[frameNumber] = newFrame;
			YerFace_MutexUnlock(self->myMutex);
			for(MarkerTracker *markerTracker : self->trackers) {
				markerTracker->frameStatusNew(frameNumber);
			}
			break;
		case FRAME_STATUS_MAPPING:
			self->logger->debug4("handleFrameStatusChange() Frame #" YERFACE_FRAMENUMBER_FORMAT " entered MAPPING.", frameNumber);
			YerFace_MutexLock(self->myMutex);
			self->pendingFrames[frameNumber].hasEnteredMapping = true;
			YerFace_MutexUnlock(self->myMutex);
			if(self->workerPool != NULL) {
				self->workerPool->sendWorkerSignal();
			}
			break;
		case FRAME_STATUS_GONE:
			YerFace_MutexLock(self->myMutex);
			self->pendingFrames.erase(frameNumber);
			YerFace_MutexUnlock(self->myMutex);
			for(MarkerTracker *markerTracker : self->trackers) {
				markerTracker->frameStatusGone(frameNumber);
			}
			break;
	}
}

bool FaceMapper::workerHandler(WorkerPoolWorker *worker) {
	FaceMapper *self = (FaceMapper *)worker->ptr;
	bool didWork = false;
	static FrameNumber lastFrameNumber = -1;

	YerFace_MutexLock(self->myMutex);
	//// CHECK FOR WORK ////
	FrameNumber myFrameNumber = -1;
	FrameNumber lowestPendingFrameNumber = -1;
	for(auto pendingFramePair : self->pendingFrames) {
		FaceMapperPendingFrame *pendingFrame = &pendingFramePair.second;
		if((lowestPendingFrameNumber < 0 || pendingFrame->frameNumber < lowestPendingFrameNumber) && !pendingFrame->hasCompletedMapping) {
			lowestPendingFrameNumber = pendingFrame->frameNumber;
		}
	}
	if(lowestPendingFrameNumber > 0 && self->pendingFrames[lowestPendingFrameNumber].hasEnteredMapping) {
		myFrameNumber = lowestPendingFrameNumber;
	}
	YerFace_MutexUnlock(self->myMutex);

	//// DO THE WORK ////
	if(myFrameNumber > 0) {
		self->logger->debug4("Thread #%d handling frame #" YERFACE_FRAMENUMBER_FORMAT, worker->num, myFrameNumber);
		if(myFrameNumber <= lastFrameNumber) {
			throw logic_error("FaceMapper handling frames out of order!");
		}
		lastFrameNumber = myFrameNumber;

		MetricsTick tick = self->metrics->startClock();
		for(MarkerTracker *tracker : self->trackers) {
			tracker->processFrame(myFrameNumber);
		}
		self->metrics->endClock(tick);

		self->frameServer->setWorkingFrameStatusCheckpoint(myFrameNumber, FRAME_STATUS_MAPPING, "faceMapper.ran");
		YerFace_MutexLock(self->myMutex);
		self->pendingFrames[myFrameNumber].hasCompletedMapping = true;
		YerFace_MutexUnlock(self->myMutex);
		didWork = true;
	}
	return didWork;
}

}; //namespace YerFace


```

## Source file: src/FaceMapper.hpp

{
  "path": "src/FaceMapper.hpp",
  "blob_sha": "c11ff3512ad17dede61f379a3b96da5bb65100e8",
  "sha256": "b4f5651cf7f0dbdf8dff9309f920f63827f4ac310232791679ea799777f31e30",
  "lines": 74,
  "bytes": 1914,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 10,
    "jaw": 1,
    "rig": 8,
    "face": 10,
    "lip": 6
  },
  "url": "https://github.com/markleybros/yer-face/blob/e3968fdb3b0c3864325716fca69cc37b38be5bad/src/FaceMapper.hpp",
  "function_signature_candidates": [
    {
      "line": 29,
      "text": "void renderPreviewHUD(cv::Mat frame, FrameNumber frameNumber, int density, bool mirrorMode);"
    },
    {
      "line": 33,
      "text": "static void handleFrameStatusChange(void *userdata, WorkingFrameStatus newStatus, FrameTimestamps frameTimestamps);"
    },
    {
      "line": 34,
      "text": "static bool workerHandler(WorkerPoolWorker *worker);"
    }
  ]
}

```text

#pragma once

#include "Logger.hpp"
#include "SDLDriver.hpp"
#include "FrameServer.hpp"
#include "FaceTracker.hpp"
#include "MarkerTracker.hpp"
#include "Metrics.hpp"
#include "Status.hpp"
#include "PreviewHUD.hpp"

using namespace std;

namespace YerFace {

class MarkerTracker;

class FaceMapperPendingFrame {
public:
	FrameNumber frameNumber;
	bool hasEnteredMapping;
	bool hasCompletedMapping;
};

class FaceMapper {
public:
	FaceMapper(json config, Status *myStatus, FrameServer *myFrameServer, FaceTracker *myFaceTracker, PreviewHUD *myPreviewHUD);
	~FaceMapper() noexcept(false);
	void renderPreviewHUD(cv::Mat frame, FrameNumber frameNumber, int density, bool mirrorMode);
	FrameServer *getFrameServer(void);
	FaceTracker *getFaceTracker(void);
private:
	static void handleFrameStatusChange(void *userdata, WorkingFrameStatus newStatus, FrameTimestamps frameTimestamps);
	static bool workerHandler(WorkerPoolWorker *worker);

	Status *status;
	FrameServer *frameServer;
	FaceTracker *faceTracker;
	PreviewHUD *previewHUD;

	Logger *logger;
	Metrics *metrics;

	MarkerTracker *markerEyelidLeftTop;
	MarkerTracker *markerEyelidRightTop;
	MarkerTracker *markerEyelidLeftBottom;
	MarkerTracker *markerEyelidRightBottom;

	MarkerTracker *markerEyebrowLeftInner;
	MarkerTracker *markerEyebrowLeftMiddle;
	MarkerTracker *markerEyebrowLeftOuter;
	MarkerTracker *markerEyebrowRightInner;
	MarkerTracker *markerEyebrowRightMiddle;
	MarkerTracker *markerEyebrowRightOuter;

	MarkerTracker *markerJaw;

	MarkerTracker *markerLipsLeftCorner;
	MarkerTracker *markerLipsRightCorner;

	MarkerTracker *markerLipsLeftTop;
	MarkerTracker *markerLipsRightTop;
	
	MarkerTracker *markerLipsLeftBottom;
	MarkerTracker *markerLipsRightBottom;

	std::vector<MarkerTracker *> trackers;

	SDL_mutex *myMutex;
	std::unordered_map<FrameNumber, FaceMapperPendingFrame> pendingFrames;
	WorkerPool *workerPool;
};

}; //namespace YerFace


```

## Source file: src/FaceTracker.cpp

{
  "path": "src/FaceTracker.cpp",
  "blob_sha": "463019327098f5dd205364837185c120914ae372",
  "sha256": "12b6ae1b03bf367a8a7eb4c7fee3f35d092e341dd22602bba38c4843a79728ca",
  "lines": 796,
  "bytes": 32775,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 16,
    "jaw": 4,
    "mouth": 7,
    "gum": 17,
    "rig": 15,
    "face": 140,
    "lip": 6
  },
  "url": "https://github.com/markleybros/yer-face/blob/e3968fdb3b0c3864325716fca69cc37b38be5bad/src/FaceTracker.cpp",
  "function_signature_candidates": []
}

```text


#include "FaceTracker.hpp"
#include "Utilities.hpp"

#include "dlib/opencv.h"
#include "dlib/dnn.h"
#include "dlib/image_processing/frontal_face_detector.h"
#include "dlib/image_processing/render_face_detections.h"
#include "dlib/image_processing.h"

#include "opencv2/calib3d.hpp"

#include <exception>
#include <cmath>
#include <cstdio>
#include <cstdlib>

using namespace std;
using namespace dlib;
using namespace cv;

namespace YerFace {

class DlibPointPointer {
public:
	dlib::point *ptr;
};

class FaceTrackerWorker {
public:
	FaceTracker *self;

	dlib::shape_predictor shapePredictor;
};


// Pose recovery approach largely informed by the following sources:
//  - https://www.learnopencv.com/head-pose-estimation-using-opencv-and-dlib/
//  - https://github.com/severin-lemaignan/gazr/
FaceTracker::FaceTracker(json config, Status *myStatus, SDLDriver *mySDLDriver, FrameServer *myFrameServer, FaceDetector *myFaceDetector) {
	predictorWorkerPool = NULL;
	assignmentWorkerPool = NULL;

	featureDetectionModelFileName = Utilities::fileValidPathOrDie(config["YerFace"]["FaceTracker"]["dlibFaceLandmarks"]);
	useFullSizedFrameForLandmarkDetection = config["YerFace"]["FaceTracker"]["useFullSizedFrameForLandmarkDetection"];
	previouslyReportedFacialPose.set = false;
	facialCameraModel.set = false;

	status = myStatus;
	if(status == NULL) {
		throw invalid_argument("status cannot be NULL");
	}
	sdlDriver = mySDLDriver;
	if(sdlDriver == NULL) {
		throw invalid_argument("sdlDriver cannot be NULL");
	}
	frameServer = myFrameServer;
	if(frameServer == NULL) {
		throw invalid_argument("frameServer cannot be NULL");
	}
	faceDetector = myFaceDetector;
	if(faceDetector == NULL) {
		throw invalid_argument("faceDetector cannot be NULL");
	}
	poseSmoothingOverSeconds = config["YerFace"]["FaceTracker"]["poseSmoothingOverSeconds"];
	if(poseSmoothingOverSeconds <= 0.0) {
		throw invalid_argument("poseSmoothingOverSeconds cannot be less than or equal to zero.");
	}
	poseSmoothingExponent = config["YerFace"]["FaceTracker"]["poseSmoothingExponent"];
	if(poseSmoothingExponent <= 0.0) {
		throw invalid_argument("poseSmoothingExponent cannot be less than or equal to zero.");
	}
	poseRotationLowRejectionThreshold = config["YerFace"]["FaceTracker"]["poseRotationLowRejectionThreshold"];
	if(poseRotationLowRejectionThreshold <= 0.0) {
		throw invalid_argument("poseRotationLowRejectionThreshold cannot be less than or equal to zero.");
	}
	poseTranslationLowRejectionThreshold = config["YerFace"]["FaceTracker"]["poseTranslationLowRejectionThreshold"];
	if(poseTranslationLowRejectionThreshold <= 0.0) {
		throw invalid_argument("poseRotationLowRejectionThreshold cannot be less than or equal to zero.");
	}
	poseRotationLowRejectionThresholdInternal = config["YerFace"]["FaceTracker"]["poseRotationLowRejectionThresholdInternal"];
	if(poseRotationLowRejectionThresholdInternal <= 0.0) {
		throw invalid_argument("poseRotationLowRejectionThresholdInternal cannot be less than or equal to zero.");
	}
	poseTranslationLowRejectionThresholdInternal = config["YerFace"]["FaceTracker"]["poseTranslationLowRejectionThresholdInternal"];
	if(poseTranslationLowRejectionThresholdInternal <= 0.0) {
		throw invalid_argument("poseRotationLowRejectionThresholdInternal cannot be less than or equal to zero.");
	}
	poseRotationHighRejectionThreshold = config["YerFace"]["FaceTracker"]["poseRotationHighRejectionThreshold"];
	if(poseRotationHighRejectionThreshold <= 0.0) {
		throw invalid_argument("poseRotationHighRejectionThreshold cannot be less than or equal to zero.");
	}
	poseTranslationHighRejectionThreshold = config["YerFace"]["FaceTracker"]["poseTranslationHighRejectionThreshold"];
	if(poseTranslationHighRejectionThreshold <= 0.0) {
		throw invalid_argument("poseRotationHighRejectionThreshold cannot be less than or equal to zero.");
	}
	poseRejectionResetAfterSeconds = config["YerFace"]["FaceTracker"]["poseRejectionResetAfterSeconds"];
	if(poseRejectionResetAfterSeconds <= 0.0) {
		throw invalid_argument("poseRejectionResetAfterSeconds cannot be less than or equal to zero.");
	}
	poseTranslationMaxX = config["YerFace"]["FaceTracker"]["poseTranslationMaxX"];
	poseTranslationMinX = config["YerFace"]["FaceTracker"]["poseTranslationMinX"];
	poseTranslationMaxY = config["YerFace"]["FaceTracker"]["poseTranslationMaxY"];
	poseTranslationMinY = config["YerFace"]["FaceTracker"]["poseTranslationMinY"];
	poseTranslationMaxZ = config["YerFace"]["FaceTracker"]["poseTranslationMaxZ"];
	poseTranslationMinZ = config["YerFace"]["FaceTracker"]["poseTranslationMinZ"];
	poseRotationPlusMinusX = config["YerFace"]["FaceTracker"]["poseRotationPlusMinusX"];
	poseRotationPlusMinusY = config["YerFace"]["FaceTracker"]["poseRotationPlusMinusY"];
	poseRotationPlusMinusZ = config["YerFace"]["FaceTracker"]["poseRotationPlusMinusZ"];
	vertexNoseSellion = Utilities::Point3dFromJSONArray((json)config["YerFace"]["FaceTracker"]["solvePnPVertices"]["vertexNoseSellion"]);
	vertexEyeRightOuterCorner = Utilities::Point3dFromJSONArray((json)config["YerFace"]["FaceTracker"]["solvePnPVertices"]["vertexEyeRightOuterCorner"]);
	vertexEyeLeftOuterCorner = Utilities::Point3dFromJSONArray((json)config["YerFace"]["FaceTracker"]["solvePnPVertices"]["vertexEyeLeftOuterCorner"]);
	vertexRightEar = Utilities::Point3dFromJSONArray((json)config["YerFace"]["FaceTracker"]["solvePnPVertices"]["vertexRightEar"]);
	vertexLeftEar = Utilities::Point3dFromJSONArray((json)config["YerFace"]["FaceTracker"]["solvePnPVertices"]["vertexLeftEar"]);
	vertexNoseTip = Utilities::Point3dFromJSONArray((json)config["YerFace"]["FaceTracker"]["solvePnPVertices"]["vertexNoseTip"]);
	vertexStommion = Utilities::Point3dFromJSONArray((json)config["YerFace"]["FaceTracker"]["solvePnPVertices"]["vertexStommion"]);
	vertexMenton = Utilities::Point3dFromJSONArray((json)config["YerFace"]["FaceTracker"]["solvePnPVertices"]["vertexMenton"]);
	depthSliceA = config["YerFace"]["FaceTracker"]["depthSlices"]["A"];
	depthSliceB = config["YerFace"]["FaceTracker"]["depthSlices"]["B"];
	depthSliceC = config["YerFace"]["FaceTracker"]["depthSlices"]["C"];
	depthSliceD = config["YerFace"]["FaceTracker"]["depthSlices"]["D"];
	depthSliceE = config["YerFace"]["FaceTracker"]["depthSlices"]["E"];
	depthSliceF = config["YerFace"]["FaceTracker"]["depthSlices"]["F"];
	depthSliceG = config["YerFace"]["FaceTracker"]["depthSlices"]["G"];
	depthSliceH = config["YerFace"]["FaceTracker"]["depthSlices"]["H"];

	logger = new Logger("FaceTracker");
	metricsPredictor = new Metrics(config, "FaceTracker.Predictor");
	metricsAssignment = new Metrics(config, "FaceTracker.Assignment");

	if((myMutex = SDL_CreateMutex()) == NULL) {
		throw runtime_error("Failed creating mutex!");
	}
	if((myAssignmentMutex = SDL_CreateMutex()) == NULL) {
		throw runtime_error("Failed creating mutex!");
	}

	//We want to know when any frame has entered various statuses.
	FrameStatusChangeEventCallback frameStatusChangeCallback;
	frameStatusChangeCallback.userdata = (void *)this;
	frameStatusChangeCallback.callback = handleFrameStatusChange;
	frameStatusChangeCallback.newStatus = FRAME_STATUS_NEW;
	frameServer->onFrameStatusChangeEvent(frameStatusChangeCallback);
	frameStatusChangeCallback.newStatus = FRAME_STATUS_TRACKING;
	frameServer->onFrameStatusChangeEvent(frameStatusChangeCallback);
	frameStatusChangeCallback.newStatus = FRAME_STATUS_GONE;
	frameServer->onFrameStatusChangeEvent(frameStatusChangeCallback);

	//We also want to introduce a checkpoint so that frames cannot TRANSITION AWAY from FRAME_STATUS_TRACKING without our blessing.
	frameServer->registerFrameStatusCheckpoint(FRAME_STATUS_TRACKING, "faceTracker.ran");

	WorkerPoolParameters workerPoolParameters;
	workerPoolParameters.name = "FaceTracker.Predictor";
	workerPoolParameters.numWorkers = config["YerFace"]["FaceTracker"]["numWorkers"];
	workerPoolParameters.numWorkersPerCPU = config["YerFace"]["FaceTracker"]["numWorkersPerCPU"];
	workerPoolParameters.initializer = predictorWorkerInitializer;
	workerPoolParameters.deinitializer = NULL;
	workerPoolParameters.usrPtr = (void *)this;
	workerPoolParameters.handler = predictorWorkerHandler;
	predictorWorkerPool = new WorkerPool(config, status, frameServer, workerPoolParameters);

	workerPoolParameters.name = "FaceTracker.Assignment";
	workerPoolParameters.numWorkers = 1;
	workerPoolParameters.numWorkersPerCPU = 0.0;
	workerPoolParameters.initializer = NULL;
	workerPoolParameters.deinitializer = NULL;
	workerPoolParameters.usrPtr = (void *)this;
	workerPoolParameters.handler = assignmentWorkerHandler;
	assignmentWorkerPool = new WorkerPool(config, status, frameServer, workerPoolParameters);

	logger->debug1("FaceTracker object constructed and ready to go!");
}

FaceTracker::~FaceTracker() noexcept(false) {
	logger->debug1("FaceTracker object destructing...");

	delete predictorWorkerPool;

	YerFace_MutexLock(myMutex);
	if(pendingPredictionFrameNumbers.size() > 0) {
		logger->err("Frames are still pending! Woe is me!");
	}
	if(outputFrames.size() > 0) {
		logger->err("Outputs are still pending! Woe is me!");
	}
	YerFace_MutexUnlock(myMutex);

	YerFace_MutexLock(myAssignmentMutex);
	if(pendingAssignmentFrameNumbers.size() > 0) {
		logger->err("Assignment Frames are still pending! Woe is me!");
	}
	YerFace_MutexUnlock(myAssignmentMutex);

	SDL_DestroyMutex(myMutex);
	SDL_DestroyMutex(myAssignmentMutex);
	delete metricsPredictor;
	delete logger;
}

void FaceTracker::doIdentifyFeatures(WorkerPoolWorker *worker, WorkingFrame *workingFrame, FaceTrackerOutput *output) {
	FaceTrackerWorker *innerWorker = (FaceTrackerWorker *)worker->ptr;
	FacialDetectionBox facialDetection = faceDetector->getFacialDetection(output->frameNumber);
	if(!facialDetection.set) {
		return;
	}

	Mat searchFrame;
	double searchFrameScaleFactor;
	Rect2d searchRect;
	if(useFullSizedFrameForLandmarkDetection) {
		searchFrame = workingFrame->frame;
		searchFrameScaleFactor = 1.0;
		searchRect = facialDetection.boxNormalSize;
	} else {
		searchFrame = workingFrame->detectionFrame;
		searchFrameScaleFactor = workingFrame->detectionScaleFactor;
		searchRect = facialDetection.box;
	}

	dlib::cv_image<dlib::bgr_pixel> dlibSearchFrame = cv_image<bgr_pixel>(searchFrame);
	dlib::rectangle dlibSearchBox = dlib::rectangle(searchRect.x, searchRect.y, (searchRect.width + searchRect.x), (searchRect.height + searchRect.y));

	full_object_detection result = innerWorker->shapePredictor(dlibSearchFrame, dlibSearchBox);

	output->facialFeatures.featuresExposed.features.clear();
	output->facialFeatures.featuresExposed.features.resize(result.num_parts());

	output->facialFeatures.features.clear();
	output->facialFeatures.features3D.clear();
	DlibPointPointer partPointer;
	dlib::point part;
	Point2d partPoint;
	for(unsigned long featureIndex = 0; featureIndex < result.num_parts(); featureIndex++) {
		part = result.part(featureIndex);
		partPointer.ptr = &part;
		if(!doConvertLandmarkPointToImagePoint(partPointer, &partPoint, searchFrameScaleFactor)) {
			return;
		}

		output->facialFeatures.featuresExposed.features[featureIndex] = partPoint;

		bool pushCorrelationPoint = false;
		switch(featureIndex) {
			case IDX_NOSE_SELLION:
				output->facialFeatures.features3D.push_back(vertexNoseSellion);
				pushCorrelationPoint = true;
				break;
			case IDX_RIGHTEYE_OUTER_CORNER:
				output->facialFeatures.features3D.push_back(vertexEyeRightOuterCorner);
				pushCorrelationPoint = true;
				break;
			case IDX_LEFTEYE_OUTER_CORNER:
				output->facialFeatures.features3D.push_back(vertexEyeLeftOuterCorner);
				pushCorrelationPoint = true;
				break;
			case IDX_JAWLINE_0:
				output->facialFeatures.features3D.push_back(vertexRightEar);
				pushCorrelationPoint = true;
				break;
			case IDX_JAWLINE_16:
				output->facialFeatures.features3D.push_back(vertexLeftEar);
				pushCorrelationPoint = true;
				break;
			case IDX_NOSE_TIP:
				output->facialFeatures.features3D.push_back(vertexNoseTip);
				pushCorrelationPoint = true;
				break;
			case IDX_JAWLINE_8:
				output->facialFeatures.features3D.push_back(vertexMenton);
				pushCorrelationPoint = true;
				break;
		}
		if(pushCorrelationPoint) {
			output->facialFeatures.features.push_back(partPoint);
		}
	}

	//Stommion needs a little extra help.
	part = result.part(IDX_MOUTHIN_CENTER_TOP);
	partPointer.ptr = &part;
	Point2d mouthTop;
	if(!doConvertLandmarkPointToImagePoint(partPointer, &mouthTop, searchFrameScaleFactor)) {
		return;
	}
	part = result.part(IDX_MOUTHIN_CENTER_BOTTOM);
	partPointer.ptr = &part;
	Point2d mouthBottom;
	if(!doConvertLandmarkPointToImagePoint(partPointer, &mouthBottom, searchFrameScaleFactor)) {
		return;
	}
	partPoint = (mouthTop + mouthTop + mouthBottom) / 3.0;
	output->facialFeatures.features.push_back(partPoint);
	output->facialFeatures.features3D.push_back(vertexStommion);
	output->facialFeatures.set = true;
	output->facialFeatures.featuresExposed.set = true;
}

void FaceTracker::doInitializeCameraModel(WorkingFrame *workingFrame) {
	//Totally fake, idealized camera.
	Size frameSize = workingFrame->frame.size();
	double focalLength = frameSize.width;
	Point2d center = Point2d(frameSize.width / 2, frameSize.height / 2);

	YerFace_MutexLock(myAssignmentMutex);
	facialCameraModel.cameraMatrix = Utilities::generateFakeCameraMatrix(focalLength, center);
	facialCameraModel.distortionCoefficients = Mat::zeros(4, 1, DataType<double>::type);
	facialCameraModel.set = true;
	YerFace_MutexUnlock(myAssignmentMutex);
}

void FaceTracker::doCalculateFacialTransformation(WorkerPoolWorker *worker, WorkingFrame *workingFrame, FaceTrackerOutput *output) {
	if(!output->facialFeatures.set) {
		previouslyReportedFacialPose.set = false;
		return;
	}

	FacialCameraModel camera = facialCameraModel;

	FrameTimestamps frameTimestamps = workingFrame->frameTimestamps;
	double frameTimestamp = frameTimestamps.startTimestamp;
	FacialPose tempPose;
	tempPose.timestamp = frameTimestamp;
	tempPose.set = false;
	Mat tempRotationVector;

	//// DO FACIAL POSE SOLUTION ////

	solvePnP(output->facialFeatures.features3D, output->facialFeatures.features, camera.cameraMatrix, camera.distortionCoefficients, tempRotationVector, tempPose.translationVector);
	tempRotationVector.at<double>(0) = tempRotationVector.at<double>(0) * -1.0;
	tempRotationVector.at<double>(1) = tempRotationVector.at<double>(1) * -1.0;
	Rodrigues(tempRotationVector, tempPose.rotationMatrix);

	//// REJECT BAD / OUT OF BOUNDS FACIAL POSES ////

	bool reportNewPose = true;
	double degreesDifference, distance, scaledRotationThreshold, scaledTranslationThreshold;
	double timeScale = (double)(frameTimestamps.estimatedEndTimestamp - frameTimestamps.startTimestamp) / (double)(1.0 / 30.0);
	if(previouslyReportedFacialPose.set) {
		scaledRotationThreshold = poseRotationHighRejectionThreshold * timeScale;
		scaledTranslationThreshold = poseTranslationHighRejectionThreshold * timeScale;
		degreesDifference = Utilities::degreesDifferenceBetweenTwoRotationMatrices(previouslyReportedFacialPose.rotationMatrix, tempPose.rotationMatrix);
		distance = Utilities::lineDistance(Point3d(tempPose.translationVector), Point3d(previouslyReportedFacialPose.translationVector));
		if(degreesDifference > scaledRotationThreshold || distance > scaledTranslationThreshold) {
			logger->info("Dropping facial pose due to high rotation (%.02lf) or high motion (%.02lf)!", degreesDifference, distance);
			reportNewPose = false;
		}
	}
	if(tempPose.translationVector.at<double>(0) < poseTranslationMinX || tempPose.translationVector.at<double>(0) > poseTranslationMaxX ||
	  tempPose.translationVector.at<double>(1) < poseTranslationMinY || tempPose.translationVector.at<double>(1) > poseTranslationMaxY ||
	  tempPose.translationVector.at<double>(2) < poseTranslationMinZ || tempPose.translationVector.at<double>(2) > poseTranslationMaxZ) {
		logger->info("Dropping facial pose due to out of bounds translation: <%.02f, %.02f, %.02f>", tempPose.translationVector.at<double>(0), tempPose.translationVector.at<double>(1), tempPose.translationVector.at<double>(2));
		reportNewPose = false;
	}
	Vec3d angles = Utilities::rotationMatrixToEulerAngles(tempPose.rotationMatrix);

	if(fabs(angles[0]) > poseRotationPlusMinusX || fabs(angles[1]) > poseRotationPlusMinusY || fabs(angles[2]) > poseRotationPlusMinusZ) {
		logger->info("Dropping facial pose due to out of bounds angle: <%.02f, %.02f, %.02f>", angles[0], angles[1], angles[2]);
		reportNewPose = false;
	}
	if(!reportNewPose) {
		if(previouslyReportedFacialPose.set) {
			if(tempPose.timestamp - previouslyReportedFacialPose.timestamp >= poseRejectionResetAfterSeconds) {
				logger->notice("Facial pose has come back bad consistantly for %.02lf seconds! Unsetting the face pose completely.", tempPose.timestamp - previouslyReportedFacialPose.timestamp);
				previouslyReportedFacialPose.set = false;
			}
			output->facialPose = previouslyReportedFacialPose;
		} else {
			output->facialPose.set = false;
		}
		return;
	}

	//// DO FACIAL POSE SMOOTHING ////

	facialPoseSmoothingBuffer.push_back(tempPose);
	while(facialPoseSmoothingBuffer.front().timestamp <= (frameTimestamp - poseSmoothingOverSeconds)) {
		facialPoseSmoothingBuffer.pop_front();
	}

	tempPose.translationVector = (Mat_<double>(3,1) << 0.0, 0.0, 0.0);
	tempPose.rotationMatrix = (Mat_<double>(3,3) <<
			0.0, 0.0, 0.0,
			0.0, 0.0, 0.0,
			0.0, 0.0, 0.0);

	double combinedWeights = 0.0;
	for(FacialPose pose : facialPoseSmoothingBuffer) {
		double progress = (pose.timestamp - (frameTimestamp - poseSmoothingOverSeconds)) / poseSmoothingOverSeconds;
		double weight = std::pow(progress, (double)poseSmoothingExponent) - combinedWeights;
		combinedWeights += weight;
		for(int j = 0; j < 3; j++) {
			tempPose.translationVector.at<double>(j) += pose.translationVector.at<double>(j) * weight;
		}
		for(int j = 0; j < 9; j++) {
			tempPose.rotationMatrix.at<double>(j) += pose.rotationMatrix.at<double>(j) * weight;
		}
	}

	tempPose.set = true;
	angles = Utilities::rotationMatrixToEulerAngles(tempPose.rotationMatrix);
	logger->debug3("Facial Pose Angle: <%.02f, %.02f, %.02f>; Translation: <%.02f, %.02f, %.02f>", angles[0], angles[1], angles[2], tempPose.translationVector.at<double>(0), tempPose.translationVector.at<double>(1), tempPose.translationVector.at<double>(2));

	//// REJECT NOISY SOLUTIONS ////

	tempPose.rotationMatrixInternal = tempPose.rotationMatrix.clone();
	tempPose.translationVectorInternal = tempPose.translationVector.clone();
	if(previouslyReportedFacialPose.set) {
		// Do de-noising (low motion rejection) first for the externally-facing matrices
		scaledRotationThreshold = poseRotationLowRejectionThreshold * timeScale;
		scaledTranslationThreshold = poseTranslationLowRejectionThreshold * timeScale;

		degreesDifference = Utilities::degreesDifferenceBetweenTwoRotationMatrices(previouslyReportedFacialPose.rotationMatrix, tempPose.rotationMatrix);
		if(degreesDifference < scaledRotationThreshold) {
			tempPose.rotationMatrix = previouslyReportedFacialPose.rotationMatrix.clone();
		}
		distance = Utilities::lineDistance(Point3d(tempPose.translationVector), Point3d(previouslyReportedFacialPose.translationVector));
		if(distance < scaledTranslationThreshold) {
			tempPose.translationVector = previouslyReportedFacialPose.translationVector.clone();
		}

		// Do de-noising (low motion rejection) again, but for the internally-facing matrices
		scaledRotationThreshold = poseRotationLowRejectionThresholdInternal * timeScale;
		scaledTranslationThreshold = poseTranslationLowRejectionThresholdInternal * timeScale;

		degreesDifference = Utilities::degreesDifferenceBetweenTwoRotationMatrices(previouslyReportedFacialPose.rotationMatrixInternal, tempPose.rotationMatrixInternal);
		if(degreesDifference < scaledRotationThreshold) {
			tempPose.rotationMatrixInternal = previouslyReportedFacialPose.rotationMatrixInternal.clone();
		}
		distance = Utilities::lineDistance(Point3d(tempPose.translationVectorInternal), Point3d(previouslyReportedFacialPose.translationVectorInternal));
		if(distance < scaledTranslationThreshold) {
			tempPose.translationVectorInternal = previouslyReportedFacialPose.translationVectorInternal.clone();
		}
	}

	output->facialPose = tempPose;
	previouslyReportedFacialPose = output->facialPose;
}

void FaceTracker::doPrecalculateFacialPlaneNormal(WorkerPoolWorker *worker, WorkingFrame *workingFrame, FaceTrackerOutput *output) {
	if(!output->facialPose.set) {
		return;
	}
	Mat planeNormalMat = (Mat_<double>(3, 1) << 0.0, 0.0, -1.0);
	planeNormalMat = output->facialPose.rotationMatrixInternal * planeNormalMat;
	output->facialPose.facialPlaneNormal = Vec3d(planeNormalMat.at<double>(0), planeNormalMat.at<double>(1), planeNormalMat.at<double>(2));
}

bool FaceTracker::doConvertLandmarkPointToImagePoint(DlibPointPointer pointPointer, Point2d *dst, double detectionScaleFactor) {
	dlib::point *src = pointPointer.ptr;
	if(*src == OBJECT_PART_NOT_PRESENT) {
		return false;
	}
	*dst = Point2d(src->x(), src->y());
	dst->x /= detectionScaleFactor;
	dst->y /= detectionScaleFactor;
	return true;
}

void FaceTracker::renderPreviewHUD(Mat frame, FrameNumber frameNumber, int density, bool mirrorMode) {
	YerFace_MutexLock(myMutex);
	if(frameNumber < 0 || outputFrames.find(frameNumber) == outputFrames.end()) {
		YerFace_MutexUnlock(myMutex);
		throw invalid_argument("FaceTracker::renderPreviewHUD() passed invalid frame number");
	}
	FaceTrackerOutput output = outputFrames[frameNumber];
	YerFace_MutexUnlock(myMutex);

	YerFace_MutexLock(myAssignmentMutex);
	FacialCameraModel camera = facialCameraModel;
	YerFace_MutexUnlock(myAssignmentMutex);

	int frameWidth = frame.size().width;

	if(density > 0) {
		if(output.facialPose.set) {
			std::vector<Point3d> gizmo3d(6);
			std::vector<Point2d> gizmo2d;
			gizmo3d[0] = Point3d(-50,0.0,0.0);
			gizmo3d[1] = Point3d(50,0.0,0.0);
			gizmo3d[2] = Point3d(0.0,50,0.0);
			gizmo3d[3] = Point3d(0.0,-50,0.0);
			gizmo3d[4] = Point3d(0.0,0.0,50);
			gizmo3d[5] = Point3d(0.0,0.0,-50);
			
			Mat tempRotationVector;
			Rodrigues(output.facialPose.rotationMatrix, tempRotationVector);
			projectPoints(gizmo3d, tempRotationVector, output.facialPose.translationVector, camera.cameraMatrix, camera.distortionCoefficients, gizmo2d);
			if(mirrorMode) {
				for(int i = 0; i <= 5; i++) {
					gizmo2d[i].x = frameWidth - gizmo2d[i].x;
				}
			}
			arrowedLine(frame, gizmo2d[0], gizmo2d[1], Scalar(0, 0, 255), 2, LINE_AA); // FIXME - proportional drawing size
			arrowedLine(frame, gizmo2d[2], gizmo2d[3], Scalar(255, 0, 0), 2, LINE_AA); // FIXME - proportional drawing size
			arrowedLine(frame, gizmo2d[4], gizmo2d[5], Scalar(0, 255, 0), 2, LINE_AA); // FIXME - proportional drawing size
		}
	}
	if(density > 3) {
		if(output.facialFeatures.set) {
			for(Point2d feature : output.facialFeatures.featuresExposed.features) {
				if(mirrorMode) {
					feature.x = frameWidth - feature.x;
				}
				Utilities::drawX(frame, feature, Scalar(147, 20, 255)); // FIXME - proportional drawing
			}
		}
	}

	if(density > 4) {
		if(output.facialPose.set) {
			std::vector<Point3d> edges3d;
			std::vector<Point2d> edges2d;

			std::list<double> depths = { depthSliceA, depthSliceB, depthSliceC, depthSliceD, depthSliceE, depthSliceF, depthSliceG };
			for(double depth : depths) {
				double planeWidth = 300;
				double gridIncrement = 25;
				double planeEdge = (planeWidth / 2.0);
				for(double x = planeEdge * -1.0; x <= planeEdge; x = x + gridIncrement) {
					edges3d.push_back(Point3d(x, planeEdge, depth));
					edges3d.push_back(Point3d(x, planeEdge * -1.0, depth));
				}
				for(double y = planeEdge * -1.0; y <= planeEdge; y = y + gridIncrement) {
					edges3d.push_back(Point3d(planeEdge, y, depth));
					edges3d.push_back(Point3d(planeEdge * -1.0, y, depth));
				}
			}

			projectPoints(edges3d, output.facialPose.rotationMatrix, output.facialPose.translationVector, camera.cameraMatrix, camera.distortionCoefficients, edges2d);

			for(unsigned int i = 0; i + 1 < edges2d.size(); i = i + 2) {
				if(mirrorMode) {
					edges2d[i].x = frameWidth - edges2d[i].x;
					edges2d[i + 1].x = frameWidth - edges2d[i + 1].x;
				}
				cv::line(frame, edges2d[i], edges2d[i + 1], Scalar(255, 255, 255), 1, LINE_AA); // FIXME - proportional drawing
			}
		}
	}
}

FacialFeatures FaceTracker::getFacialFeatures(FrameNumber frameNumber) {
	FacialFeatures val;
	YerFace_MutexLock(myMutex);
	if(frameNumber < 0 || outputFrames.find(frameNumber) == outputFrames.end()) {
		YerFace_MutexUnlock(myMutex);
		throw invalid_argument("FaceTracker::getFacialFeatures() passed invalid frame number");
	}
	val = outputFrames[frameNumber].facialFeatures.featuresExposed;
	YerFace_MutexUnlock(myMutex);
	return val;
}

FacialCameraModel FaceTracker::getFacialCameraModel(void) {
	FacialCameraModel val;
	YerFace_MutexLock(myAssignmentMutex);
	val = facialCameraModel;
	YerFace_MutexUnlock(myAssignmentMutex);
	return val;
}

FacialPose FaceTracker::getFacialPose(FrameNumber frameNumber) {
	FacialPose val;
	YerFace_MutexLock(myMutex);
	if(frameNumber < 0 || outputFrames.find(frameNumber) == outputFrames.end()) {
		YerFace_MutexUnlock(myMutex);
		throw invalid_argument("FaceTracker::getFacialPose() passed invalid frame number");
	}
	val = outputFrames[frameNumber].facialPose;
	YerFace_MutexUnlock(myMutex);
	return val;
}

FacialPlane FaceTracker::getCalculatedFacialPlaneForWorkingFacialPose(FrameNumber frameNumber, MarkerType markerType) {
	FacialPose facialPose;
	YerFace_MutexLock(myMutex);
	if(frameNumber < 0 || outputFrames.find(frameNumber) == outputFrames.end()) {
		YerFace_MutexUnlock(myMutex);
		throw invalid_argument("FaceTracker::getCalculatedFacialPlaneForWorkingFacialPose() passed invalid frame number");
	}
	facialPose = outputFrames[frameNumber].facialPose;
	YerFace_MutexUnlock(myMutex);

	if(!facialPose.set) {
		throw runtime_error("Can't do FaceTracker::getCalculatedFacialPlaneForWorkingFacialPose() when no working FacialPose is set.");
	}

	double depth = 0;
	switch(markerType.type) {
		default:
			throw runtime_error("Unsupported MarkerType!");
		case EyelidLeftTop:
		case EyelidLeftBottom:
		case EyelidRightTop:
		case EyelidRightBottom:
			depth = depthSliceG;
			break;
		case EyebrowLeftInner:
		case EyebrowRightInner:
			depth = depthSliceE;
			break;
		case EyebrowLeftMiddle:
		case EyebrowRightMiddle:
			depth = depthSliceF;
			break;
		case EyebrowLeftOuter:
		case EyebrowRightOuter:
			depth = depthSliceH;
			break;
		case LipsLeftCorner:
		case LipsRightCorner:
			depth = depthSliceD;
			break;
		case LipsLeftTop:
		case LipsRightTop:
			depth = depthSliceC;
			break;
		case LipsLeftBottom:
		case LipsRightBottom:
			depth = depthSliceB;
			break;
		case Jaw:
			depth = depthSliceA;
			break;
	}

	Mat translationOffset = (Mat_<double>(3,1) << 0.0, 0.0, depth);
	translationOffset = facialPose.rotationMatrixInternal * translationOffset;

	FacialPlane facialPlane;
	Mat translationVector = facialPose.translationVectorInternal + translationOffset;
	facialPlane.planePoint = Point3d(translationVector.at<double>(0), translationVector.at<double>(1), translationVector.at<double>(2));
	facialPlane.planeNormal = facialPose.facialPlaneNormal;

	return facialPlane;
}

void FaceTracker::handleFrameStatusChange(void *userdata, WorkingFrameStatus newStatus, FrameTimestamps frameTimestamps) {
	FrameNumber frameNumber = frameTimestamps.frameNumber;
	FaceTracker *self = (FaceTracker *)userdata;
	FaceTrackerOutput output;
	FaceTrackerAssignmentTask assignment;
	switch(newStatus) {
		default:
			throw logic_error("Handler passed unsupported frame status change event!");
		case FRAME_STATUS_NEW:
			output.set = false;
			output.facialFeatures.set = false;
			output.facialFeatures.featuresExposed.set = false;
			output.facialPose.set = false;
			YerFace_MutexLock(self->myMutex);
			self->outputFrames[frameNumber] = output;
			YerFace_MutexUnlock(self->myMutex);
			assignment.frameNumber = frameNumber;
			assignment.readyForAssignment = false;
			YerFace_MutexLock(self->myAssignmentMutex);
			self->pendingAssignmentFrameNumbers[frameNumber] = assignment;
			YerFace_MutexUnlock(self->myAssignmentMutex);
			break;
		case FRAME_STATUS_TRACKING:
			self->logger->debug4("handleFrameStatusChange() Frame #" YERFACE_FRAMENUMBER_FORMAT " waiting on me. Queue depth is now %lu", frameNumber, self->pendingPredictionFrameNumbers.size());
			YerFace_MutexLock(self->myMutex);
			self->pendingPredictionFrameNumbers.push_back(frameNumber);
			YerFace_MutexUnlock(self->myMutex);
			if(self->predictorWorkerPool != NULL) {
				self->predictorWorkerPool->sendWorkerSignal();
			}
			break;
		case FRAME_STATUS_GONE:
			YerFace_MutexLock(self->myMutex);
			self->outputFrames.erase(frameNumber);
			YerFace_MutexUnlock(self->myMutex);
			break;
	}
}

void FaceTracker::predictorWorkerInitializer(WorkerPoolWorker *worker, void *ptr) {
	FaceTracker *self = (FaceTracker *)ptr;
	FaceTrackerWorker *innerWorker = new FaceTrackerWorker();
	innerWorker->self = self;
	deserialize(self->featureDetectionModelFileName.c_str()) >> innerWorker->shapePredictor;
	worker->ptr = (void *)innerWorker;
}

bool FaceTracker::predictorWorkerHandler(WorkerPoolWorker *worker) {
	FaceTrackerWorker *innerWorker = (FaceTrackerWorker *)worker->ptr;
	FaceTracker *self = innerWorker->self;

	bool didWork = false;
	FrameNumber myFrameNumber = -1;

	YerFace_MutexLock(self->myMutex);
	//// CHECK FOR WORK ////
	if(self->pendingPredictionFrameNumbers.size() > 0) {
		myFrameNumber = self->pendingPredictionFrameNumbers.front();
		self->pendingPredictionFrameNumbers.pop_front();
	}
	YerFace_MutexUnlock(self->myMutex);

	//// DO THE WORK ////
	if(myFrameNumber > 0) {
		MetricsTick tick = self->metricsPredictor->startClock();

		WorkingFrame *workingFrame = self->frameServer->getWorkingFrame(myFrameNumber);

		FaceTrackerOutput output;
		output.set = false;
		output.facialFeatures.set = false;
		output.facialFeatures.featuresExposed.set = false;
		output.facialPose.set = false;
		output.frameNumber = myFrameNumber;

		self->doIdentifyFeatures(worker, workingFrame, &output);

		YerFace_MutexLock(self->myMutex);
		self->outputFrames[myFrameNumber] = output;
		YerFace_MutexUnlock(self->myMutex);

		YerFace_MutexLock(self->myAssignmentMutex);
		self->pendingAssignmentFrameNumbers[myFrameNumber].readyForAssignment = true;
		YerFace_MutexUnlock(self->myAssignmentMutex);
		if(self->assignmentWorkerPool != NULL) {
			self->assignmentWorkerPool->sendWorkerSignal();
		}

		self->metricsPredictor->endClock(tick);
		didWork = true;
	}

	return didWork;
}

bool FaceTracker::assignmentWorkerHandler(WorkerPoolWorker *worker) {
	FaceTracker *self = (FaceTracker *)worker->ptr;

	bool didWork = false;
	FrameNumber myFrameNumber = -1;
	static FrameNumber lastFrameNumber = -1;

	YerFace_MutexLock(self->myAssignmentMutex);
	//// CHECK FOR WORK ////
	for(auto pendingAssignmentPair : self->pendingAssignmentFrameNumbers) {
		if(myFrameNumber < 0 || pendingAssignmentPair.first < myFrameNumber) {
			myFrameNumber = pendingAssignmentPair.first;
		}
	}
	if(myFrameNumber > 0 && !self->pendingAssignmentFrameNumbers[myFrameNumber].readyForAssignment) {
		self->logger->debug4("BLOCKED on frame " YERFACE_FRAMENUMBER_FORMAT " because it is not ready!", myFrameNumber);
		myFrameNumber = -1;
	}
	if(myFrameNumber > 0) {
		self->pendingAssignmentFrameNumbers.erase(myFrameNumber);
	}
	YerFace_MutexUnlock(self->myAssignmentMutex);

	YerFace_MutexLock(self->myMutex);
	FaceTrackerOutput output;
	if(myFrameNumber > 0) {
		output = self->outputFrames[myFrameNumber];
	}
	YerFace_MutexUnlock(self->myMutex);

	//// DO THE WORK ////
	if(myFrameNumber > 0) {
		self->logger->debug4("Face Tracker Assignment Thread handling frame #" YERFACE_FRAMENUMBER_FORMAT, myFrameNumber);
		if(myFrameNumber <= lastFrameNumber) {
			throw logic_error("FaceTracker handling frames out of order!");
		}
		lastFrameNumber = myFrameNumber;

		MetricsTick tick = self->metricsAssignment->startClock();

		WorkingFrame *workingFrame = self->frameServer->getWorkingFrame(myFrameNumber);
		
		YerFace_MutexLock(self->myAssignmentMutex);
		if(!self->facialCameraModel.set) {
			self->doInitializeCameraModel(workingFrame);
		}
		self->doCalculateFacialTransformation(worker, workingFrame, &output);
		self->doPrecalculateFacialPlaneNormal(worker, workingFrame, &output);
		YerFace_MutexUnlock(self->myAssignmentMutex);

		YerFace_MutexLock(self->myMutex);
		self->outputFrames[myFrameNumber] = output;
		YerFace_MutexUnlock(self->myMutex);

		self->frameServer->setWorkingFrameStatusCheckpoint(myFrameNumber, FRAME_STATUS_TRACKING, "faceTracker.ran");
		self->metricsAssignment->endClock(tick);

		didWork = true;
	}

	return didWork;
}

}; //namespace YerFace


```
