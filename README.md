# subwallet

A simple Command Line Interface Wallet for Polkadot/Substrate.

<img src="https://raw.githubusercontent.com/w3f/Open-Grants-Program/master/src/web3%20foundation%20grants_black.jpg" width="300px">


## Installation

TODO


## Usage

```bash
$ ./subwallet -h
subwallet 0.1.0
yxf <yxf4559@gmail.com>
A simple Command Line Interface wallet for Polkadot/Substrate.

USAGE:
    subwallet [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    backup           Backup specified address to local json file
    getnewaddress    Generate a new address associated with label, deafult cryptography is sr25519
    help             Prints this message or the help of the given subcommand(s)
    listaddresses    Prints the list of addresses
    restore          Restore address from json file
```

### Subcommands

`./subwallet help [SUBCOMMAND]` to prints help information of subcommand.

#### `getnewaddress`

Generate a new random address

Example:

```bash
$ ./subwallet getnewaddress demo
1c1AVSCrrhtakya7LSm2hXHJUmBFdXV2KmCgEgDDaTWUQK3
```


#### `listaddresses`

List all generated addresses

Example:
``` bash
$ ./subwallet listaddresses
ec              1EE8Q6nt4x3x3Cm9eevvtCBesEUfwTJ4bw4ocQUkNrd42Z1j        ecdsa
demo            15FarxkDPL7LPvBPd1RDMGugGFs8be2ijuHEuLJd9z67PdNm        sr25519
ed              16Q55taKB1ggt3VgQ8EFTRkmYTgtNKb9xka8hqMqXMPLCNxU        ed25519
```

#### `restore`

Restore address from json file. It is compatible with keystore file generated on [`https://polkadot.js.org/apps`](https://polkadot.js.org/apps).

Example:
``` bash
./subwallet restore ~/1EE8Q6nt4x3x3Cm9eevvtCBesEUfwTJ4bw4ocQUkNrd42Z1j.json
Password: #Type password to decode seed
1EE8Q6nt4x3x3Cm9eevvtCBesEUfwTJ4bw4ocQUkNrd42Z1j is restored
```
#### `backup` 

Backup address to local json file. The backed file can be restored on [`https://polkadot.js.org/apps`](https://polkadot.js.org/apps).

Example:
``` bash
./subwallet backup demo ~/demo.json
Type password to encrypt seed: # password
Address `15FarxkDPL7LPvBPd1RDMGugGFs8be2ijuHEuLJd9z67PdNm` is backed up to file `~/demo.json`
```



## Contributing
Bug reports and pull requests are welcome on GitHub at https://github.com/yxf/subwallet


## License
MIT
