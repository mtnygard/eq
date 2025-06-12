use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "eq")]
#[command(about = "Command-line EDN processor")]
#[command(version)]
pub struct Args {
    /// Filter expression to apply
    pub filter: String,
    
    /// Input files (reads from stdin if none provided)
    pub files: Vec<PathBuf>,
    
    /// Compact instead of pretty-printed output
    #[arg(short = 'c', long)]
    pub compact: bool,
    
    /// Output raw strings, not EDN strings
    #[arg(short = 'r', long)]
    pub raw_output: bool,
    
    /// Each line of input is a string, not parsed as EDN
    #[arg(short = 'R', long)]
    pub raw_input: bool,
    
    /// Read entire input stream into array
    #[arg(short = 's', long)]
    pub slurp: bool,
    
    /// Don't read input; filter gets nil input
    #[arg(short = 'n', long)]
    pub null_input: bool,
    
    /// Set exit status based on output
    #[arg(short = 'e', long)]
    pub exit_status: bool,
    
    /// Read filter from file
    #[arg(short = 'f', long, value_name = "FILE")]
    pub from_file: Option<PathBuf>,
    
    /// Use tabs for indentation
    #[arg(long)]
    pub tab: bool,
    
    /// Use n spaces for indentation
    #[arg(long, value_name = "N", default_value = "2")]
    pub indent: usize,
    
    /// Show debug information
    #[arg(long)]
    pub debug: bool,
    
    /// Verbose output
    #[arg(short = 'v', long)]
    pub verbose: bool,
    
    /// Print filename for each output line (like grep -H)
    #[arg(short = 'H', long)]
    pub with_filename: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Args::command().debug_assert()
    }
    
    #[test]
    fn test_basic_args() {
        let args = Args::try_parse_from(&["eq", "."]).unwrap();
        assert_eq!(args.filter, ".");
        assert!(args.files.is_empty());
        assert!(!args.compact);
    }
    
    #[test]
    fn test_file_args() {
        let args = Args::try_parse_from(&["eq", "(first)", "test.edn"]).unwrap();
        assert_eq!(args.filter, "(first)");
        assert_eq!(args.files.len(), 1);
        assert_eq!(args.files[0], PathBuf::from("test.edn"));
    }
    
    #[test]
    fn test_flags() {
        let args = Args::try_parse_from(&["eq", "-c", "-r", "--tab", "."]).unwrap();
        assert!(args.compact);
        assert!(args.raw_output);
        assert!(args.tab);
    }
    
    #[test]
    fn test_with_filename_flag() {
        let args = Args::try_parse_from(&["eq", "-H", ".", "file1.edn"]).unwrap();
        assert!(args.with_filename);
        
        let args = Args::try_parse_from(&["eq", "--with-filename", ".", "file1.edn"]).unwrap();
        assert!(args.with_filename);
    }
}