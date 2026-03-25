import { VideoTranscoder } from './VideoTranscoder';
import { spawn } from 'child_process';
import { promises as fsPromises } from 'fs';
import { S3Client } from '@aws-sdk/client-s3';
import { EventEmitter } from 'events';

// --- Mock Dependencies ---
jest.mock('child_process', () => ({
  spawn: jest.fn(),
}));

jest.mock('fs', () => ({
  createReadStream: jest.fn().mockReturnValue('mocked-stream'),
  promises: {
    mkdir: jest.fn().mockResolvedValue(undefined),
    readdir: jest.fn().mockResolvedValue(['master.m3u8', '1080p_playlist.m3u8', '1080p_sequence_0.ts']),
    rm: jest.fn().mockResolvedValue(undefined),
  },
}));

jest.mock('@aws-sdk/client-s3', () => {
  return {
    S3Client: jest.fn().mockImplementation(() => ({
      send: jest.fn().mockResolvedValue({}),
    })),
    PutObjectCommand: jest.fn().mockImplementation((args) => args),
  };
});

describe('VideoTranscoder Pipeline', () => {
  let transcoder: VideoTranscoder;
  const outputDir = '/tmp/hls-out';
  const inputPath = '/tmp/raw/upload.mp4';
  const s3Prefix = 'videos/creator123';

  beforeEach(() => {
    jest.clearAllMocks();
    transcoder = new VideoTranscoder('us-east-1', 'substream-video-bucket');
  });

  it('should process a video to HLS, upload to S3, and cleanup local files', async () => {
    // Setup mock FFmpeg process
    const mockFfmpegProcess = new EventEmitter() as any;
    mockFfmpegProcess.stderr = new EventEmitter();
    (spawn as jest.Mock).mockReturnValue(mockFfmpegProcess);

    // Execute the transcoder asynchronously
    const processPromise = transcoder.processAndUpload(inputPath, outputDir, s3Prefix);

    // Simulate FFmpeg finishing successfully
    setTimeout(() => mockFfmpegProcess.emit('close', 0), 10);

    await processPromise;

    // 1. Verify Directory was created
    expect(fsPromises.mkdir).toHaveBeenCalledWith(outputDir, { recursive: true });

    // 2. Verify FFmpeg was spawned with expected HLS and Resolution arguments
    expect(spawn).toHaveBeenCalledWith('ffmpeg', expect.arrayContaining([
      '-i', inputPath,
      '-s:v:0', '1920x1080', // 1080p
      '-s:v:1', '1280x720',  // 720p
      '-s:v:2', '854x480',   // 480p
      '-master_pl_name', 'master.m3u8',
      '-f', 'hls'
    ]));

    // 3. Verify S3 client was invoked for all mocked directory files
    // Since we mocked 3 files in fsPromises.readdir, we expect 3 S3 uploads
    expect(S3Client).toHaveBeenCalledTimes(1);
    const s3Instance = (S3Client as jest.Mock).mock.results[0].value;
    expect(s3Instance.send).toHaveBeenCalledTimes(3);

    // 4. Verify cleanup was triggered
    expect(fsPromises.rm).toHaveBeenCalledWith(outputDir, { recursive: true, force: true });
  });

  it('should throw an error and still cleanup if FFmpeg fails', async () => {
    const mockFfmpegProcess = new EventEmitter() as any;
    mockFfmpegProcess.stderr = new EventEmitter();
    (spawn as jest.Mock).mockReturnValue(mockFfmpegProcess);

    const processPromise = transcoder.processAndUpload(inputPath, outputDir, s3Prefix);
    
    // Simulate FFmpeg failing
    setTimeout(() => mockFfmpegProcess.emit('close', 1), 10);

    await expect(processPromise).rejects.toThrow('FFmpeg process exited with code 1');
    expect(fsPromises.rm).toHaveBeenCalledWith(outputDir, { recursive: true, force: true });
  });
});