use thiserror::Error;

// Custom errors for Bitcoin operations
#[derive(Error, Debug)]
pub enum BitcoinError {
    #[error("Invalid transaction format")]
    InvalidTransaction,
    #[error("Invalid script format")]
    InvalidScript,
    #[error("Invalid amount")]
    InvalidAmount,
    #[error("Parse error: {0}")]
    ParseError(String),
}

// Generic Point struct for Bitcoin addresses or coordinates
#[derive(Debug, Clone, PartialEq)]
pub struct Point<T> {
    pub x: T,
    pub y: T,
}

impl<T> Point<T> {
    pub fn new(x: T, y: T) -> Self {
        Point { x, y }
    }
}

// Custom serialization for Bitcoin transaction
pub trait BitcoinSerialize {
    fn serialize(&self) -> Vec<u8> {
        // Implement serialization to bytes
        Vec::new()
    }
}

// Legacy Bitcoin transaction
#[derive(Debug, Clone)]
pub struct LegacyTransaction {
    pub version: i32,
    pub inputs: Vec<TxInput>,
    pub outputs: Vec<TxOutput>,
    pub lock_time: u32,
}

impl LegacyTransaction {
    pub fn builder() -> LegacyTransactionBuilder {
        // Return a new builder for constructing a transaction
        LegacyTransactionBuilder::new()
    }
}

// Transaction builder
pub struct LegacyTransactionBuilder {
    pub version: i32,
    pub inputs: Vec<TxInput>,
    pub outputs: Vec<TxOutput>,
    pub lock_time: u32,
}

impl Default for LegacyTransactionBuilder {
    fn default() -> Self {
        // Implement default values
        // Initialize new builder by calling default
        Self {
            version: 1,
            inputs: Vec::new(),
            outputs: Vec::new(),
            lock_time: 0,
        }
    }
}

impl LegacyTransactionBuilder {
    pub fn new() -> Self {
        // Initialize new builder by calling default
        Self::default()
    }

    pub fn version(mut self, version: i32) -> Self {
        // Set the transaction version
        self.version = version;
        self
    }

    pub fn add_input(mut self, input: TxInput) -> Self {
        // Add input to the transaction
        self.inputs.push(input);
        self
    }

    pub fn add_output(mut self, output: TxOutput) -> Self {
        // Add output to the transaction
        self.outputs.push(output);
        self
    }

    pub fn lock_time(mut self, lock_time: u32) -> Self {
        // Set lock_time for transaction
        self.lock_time = lock_time;
        self
    }

    pub fn build(self) -> LegacyTransaction {
        // Build and return the final LegacyTransaction
        LegacyTransaction {
            version: self.version,
            inputs: self.inputs,
            outputs: self.outputs,
            lock_time: self.lock_time,
        }
    }
}

// Transaction components
#[derive(Debug, Clone)]
pub struct TxInput {
    pub previous_output: OutPoint,
    pub script_sig: Vec<u8>,
    pub sequence: u32,
}

#[derive(Debug, Clone)]
pub struct TxOutput {
    pub value: u64, // in satoshis
    pub script_pubkey: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct OutPoint {
    pub txid: [u8; 32],
    pub vout: u32,
}

// Simple CLI argument parser
pub fn parse_cli_args(args: &[String]) -> Result<CliCommand, BitcoinError> {
    // Match args to "send" or "balance" commands and parse required arguments
    // Send needs two args - amount and address
    // Balance needs no args
    if args.is_empty(){
        return Err(BitcoinError::ParseError("Missing command".to_string()));
    }
    match args[0].as_str() {
        "send" => {
            if args.len() < 3 {
                return Err(BitcoinError::ParseError(
                    "Invalid number of args.".to_string(),
                ));
            }

            let amount = args[1]
                .parse::<u64>()
                .map_err(|_| BitcoinError::InvalidAmount)?;

            let address = args[2].clone();

            Ok(CliCommand::Send { amount, address })
        }
        "balance" => {
            if args.len() > 1 {
                return Err(BitcoinError::ParseError(
                    "Balance takes no arguments".to_string(),
                ));
            }

            Ok(CliCommand::Balance)
        }
        _ => Err(BitcoinError::ParseError(format!(
            "Unknown command: {}",
            args[0]
        ))),
    }
}

pub enum CliCommand {
    Send { amount: u64, address: String },
    Balance,
}

// Decoding legacy transaction
impl TryFrom<&[u8]> for LegacyTransaction {
    type Error = BitcoinError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        // Parse binary data into a LegacyTransaction
        // Minimum length is 10 bytes (4 version + 4 inputs count + 4 lock_time)
        if data.len() < 10 {
            return Err(BitcoinError::InvalidTransaction);
        }
        let mut last_idx = 0;

        let version = i32::from_le_bytes(data[0..4].try_into().unwrap());
        last_idx += 4;
        let mut inputs: Vec<TxInput> = Vec::new();
        let (input_count, bytes_read) = parse_compact_size(&data[4..])?;
        let input_count = input_count as u32;
        last_idx += bytes_read;

        for _ in 0..input_count {
            let txid: [u8; 32] = data[last_idx..last_idx + 32]
                .try_into()
                .map_err(|_| BitcoinError::InvalidTransaction)?;
            last_idx += 32;

            let vout = u32::from_le_bytes(data[last_idx..last_idx + 4].try_into().unwrap());
            last_idx += 4;

            let previous_output = OutPoint { txid, vout };

            // Parse the script sig here ....but then again
            // is this real?
            let (script_len, bytes_read) = parse_compact_size(&data[last_idx..])?;
            last_idx += bytes_read;
            let script_sig = data[last_idx..last_idx + script_len].to_vec();
            last_idx += script_len;

            // Parse sequence (4 bytes)
            let sequence = u32::from_le_bytes(data[last_idx..last_idx + 4].try_into().unwrap());
            last_idx += 4;

            let input = TxInput {
                previous_output,
                script_sig,
                sequence,
            };

            inputs.push(input);
        }

        let mut outputs: Vec<TxOutput> = Vec::new();
        let (output_count, bytes_read) = parse_compact_size(&data[last_idx..])?;
        last_idx += bytes_read;
        for _ in 0..output_count {
            let value = u64::from_le_bytes(data[last_idx..last_idx + 8].try_into().unwrap());

            // Parse the script sig here ....but then again
            // is this real?
            let (script_len, bytes_read) = parse_compact_size(&data[last_idx..])?;
            last_idx += bytes_read;

            let script_pubkey = data[last_idx..last_idx + script_len].to_vec();
            last_idx += script_len;

            let output = TxOutput {
                value,
                script_pubkey,
            };

            outputs.push(output);
        }

        let lock_time = u32::from_le_bytes(data[last_idx..last_idx + 4].try_into().unwrap());

        Ok(LegacyTransaction {
            version,
            inputs,
            outputs,
            lock_time,
        })
    }
}

pub fn parse_compact_size(data: &[u8]) -> Result<(usize, usize), BitcoinError> {
    if data.is_empty() {
        return Err(BitcoinError::InvalidTransaction);
    }
    match data[0] {
        0xFD => {
            if data.len() < 3 {
                return Err(BitcoinError::ParseError(
                    "Not enough bytes for the particular CompactType".to_string(),
                ));
            }
            let value = u16::from_le_bytes(data[1..3].try_into().unwrap());
            Ok((value as usize, 3))
        }

        0xFE => {
            if data.len() < 5 {
                return Err(BitcoinError::ParseError(
                    "Not enough bytes for the particular CompactType".to_string(),
                ));
            }
            let value = u32::from_le_bytes(data[1..5].try_into().unwrap());
            Ok((value as usize, 5))
        }

        0xFF => {
            if data.len() < 9 {
                return Err(BitcoinError::ParseError(
                    "Not enough bytes for the particular CompactType".to_string(),
                ));
            }
            let value = u64::from_le_bytes(data[1..9].try_into().unwrap());
            Ok((value as usize, 9))
        }

        _ => Ok((data[0] as usize, 1)),
    }
}

// Custom serialization for transaction
impl BitcoinSerialize for LegacyTransaction {
    fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();

        result.extend_from_slice(&self.version.to_le_bytes());
        result.extend_from_slice(&self.lock_time.to_le_bytes());

        result
    }
}
