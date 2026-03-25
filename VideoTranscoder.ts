import { spawn } from 'child_process';
import { createReadStream, promises as fsPromises } from 'fs';
import * as path from 'path';
import { S3Client, PutObjectCommand } from '@aws-sdk/client-s3';

export class VideoTranscoder {
  private s3Client: S3Client;
  private bucketName: string;

  constructor(region: string, bucketName: string) {
    this.s3Client = new S3Client({ region });
    this.bucketName = bucketName;
  }

  /**
   * Process a video to adaptive HLS (1080p, 720p, 480p) and upload to S3.
   * @param inputFilePath Absolute path to the raw uploaded video
   * @param outputDir Temporary local directory to store HLS segments
   * @param s3DestinationPrefix S3 folder path (e.g., 'videos/user123/stream456')
   */
  public async processAndUpload(
    inputFilePath: string,
    outputDir: string,
    s3DestinationPrefix: string
  ): Promise<void> {
    try {
      // 1. Ensure output directory exists
      await fsPromises.mkdir(outputDir, { recursive: true });

      // 2. Transcode to HLS using FFmpeg
      await this.transcodeToHLS(inputFilePath, outputDir);

      // 3. Upload outputs to S3
      await this.uploadDirectoryToS3(outputDir, s3DestinationPrefix);

    } finally {
      // 4. Cleanup local files
      await fsPromises.rm(outputDir, { recursive: true, force: true }).catch(() => {
        console.warn(`Failed to clean up temp directory: ${outputDir}`);
      });
    }
  }

  /**
   * Generates 1080p, 720p, and 480p HLS playlists using FFmpeg.
   */
  private transcodeToHLS(inputFilePath: string, outputDir: string): Promise<void> {
    return new Promise((resolve, reject) => {
      const args = [
        '-y', // Overwrite existing files
        '-i', inputFilePath,
        '-preset', 'veryfast', // Optimization for processing speed
        '-g', '48', // Keyframe interval (assuming ~24fps, keyframe every 2 secs)
        '-sc_threshold', '0',
        
        // Map video and audio streams 3 times for our 3 variants
        '-map', '0:v:0', '-map', '0:a:0',
        '-map', '0:v:0', '-map', '0:a:0',
        '-map', '0:v:0', '-map', '0:a:0',
        
        // Variant 0: 1080p
        '-s:v:0', '1920x1080', '-c:v:0', 'libx264', '-b:v:0', '5000k',
        // Variant 1: 720p
        '-s:v:1', '1280x720', '-c:v:1', 'libx264', '-b:v:1', '2800k',
        // Variant 2: 480p
        '-s:v:2', '854x480', '-c:v:2', 'libx264', '-b:v:2', '1400k',
        
        // Audio settings (standardized for all variants)
        '-c:a', 'aac', '-b:a', '128k',
        
        // Define the stream mapping for the master playlist
        '-var_stream_map', 'v:0,a:0,name:1080p v:1,a:1,name:720p v:2,a:2,name:480p',
        
        // HLS output configuration
        '-master_pl_name', 'master.m3u8',
        '-f', 'hls',
        '-hls_time', '4', // 4 second segment duration
        '-hls_playlist_type', 'vod',
        '-hls_segment_filename', path.join(outputDir, '%v_sequence_%d.ts'),
        path.join(outputDir, '%v_playlist.m3u8')
      ];

      const ffmpeg = spawn('ffmpeg', args);

      ffmpeg.stderr.on('data', (data) => {
        // FFmpeg writes progress to stderr. Can be logged in trace mode if needed.
      });

      ffmpeg.on('close', (code) => {
        if (code === 0) {
          resolve();
        } else {
          reject(new Error(`FFmpeg process exited with code ${code}`));
        }
      });

      ffmpeg.on('error', (err) => {
        reject(new Error(`Failed to start FFmpeg: ${err.message}`));
      });
    });
  }

  /**
   * Uploads the generated `.m3u8` and `.ts` files to the configured S3 Bucket.
   */
  private async uploadDirectoryToS3(dirPath: string, prefix: string): Promise<void> {
    const files = await fsPromises.readdir(dirPath);
    
    const uploadPromises = files.map(async (file) => {
      const filePath = path.join(dirPath, file);
      const s3Key = path.posix.join(prefix, file);
      
      let contentType = 'application/octet-stream';
      if (file.endsWith('.m3u8')) {
        contentType = 'application/x-mpegURL';
      } else if (file.endsWith('.ts')) {
        contentType = 'video/MP2T';
      }

      const command = new PutObjectCommand({
        Bucket: this.bucketName,
        Key: s3Key,
        Body: createReadStream(filePath),
        ContentType: contentType,
      });

      await this.s3Client.send(command);
    });

    // Upload all segments and playlists in parallel
    await Promise.all(uploadPromises);
  }
}