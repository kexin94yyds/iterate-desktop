# Optional bundled audio

The public desktop source distribution intentionally contains no bundled sound
effects. The directory remains present because the Rust asset loader supports
an empty embedded-audio set.

Users can import a local notification sound in Settings. The selected file is
validated and copied into the operating system's application-data directory;
the repository and application configuration do not retain its original path.
