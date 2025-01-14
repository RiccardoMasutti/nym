// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::process;

use crate::config::Config;
use clap::ArgMatches;
use colored::Colorize;
use crypto::bech32_address_validation;
use url::Url;

pub(crate) mod describe;
pub(crate) mod init;
pub(crate) mod node_details;
pub(crate) mod run;
pub(crate) mod sign;
pub(crate) mod upgrade;

pub(crate) const ID_ARG_NAME: &str = "id";
pub(crate) const HOST_ARG_NAME: &str = "host";
pub(crate) const MIX_PORT_ARG_NAME: &str = "mix-port";
pub(crate) const VERLOC_PORT_ARG_NAME: &str = "verloc-port";
pub(crate) const HTTP_API_PORT_ARG_NAME: &str = "http-api-port";
pub(crate) const VALIDATORS_ARG_NAME: &str = "validators";
pub(crate) const ANNOUNCE_HOST_ARG_NAME: &str = "announce-host";
pub(crate) const WALLET_ADDRESS: &str = "wallet-address";

fn parse_validators(raw: &str) -> Vec<Url> {
    raw.split(',')
        .map(|raw_validator| {
            raw_validator
                .trim()
                .parse()
                .expect("one of the provided validator api urls is invalid")
        })
        .collect()
}

pub(crate) fn override_config(mut config: Config, matches: &ArgMatches) -> Config {
    let mut was_host_overridden = false;
    if let Some(host) = matches.value_of(HOST_ARG_NAME) {
        config = config.with_listening_address(host);
        was_host_overridden = true;
    }

    if let Some(port) = matches
        .value_of(MIX_PORT_ARG_NAME)
        .map(|port| port.parse::<u16>())
    {
        if let Err(err) = port {
            // if port was overridden, it must be parsable
            panic!("Invalid mix port value provided - {:?}", err);
        }
        config = config.with_mix_port(port.unwrap());
    }

    if let Some(port) = matches
        .value_of(VERLOC_PORT_ARG_NAME)
        .map(|port| port.parse::<u16>())
    {
        if let Err(err) = port {
            // if port was overridden, it must be parsable
            panic!("Invalid verloc port value provided - {:?}", err);
        }
        config = config.with_verloc_port(port.unwrap());
    }

    if let Some(port) = matches
        .value_of(HTTP_API_PORT_ARG_NAME)
        .map(|port| port.parse::<u16>())
    {
        if let Err(err) = port {
            // if port was overridden, it must be parsable
            panic!("Invalid http api port value provided - {:?}", err);
        }
        config = config.with_http_api_port(port.unwrap());
    }

    if let Some(raw_validators) = matches.value_of(VALIDATORS_ARG_NAME) {
        config = config.with_custom_validator_apis(parse_validators(raw_validators));
    }

    if let Some(announce_host) = matches.value_of(ANNOUNCE_HOST_ARG_NAME) {
        config = config.with_announce_address(announce_host);
    } else if was_host_overridden {
        // make sure our 'announce-host' always defaults to 'host'
        config = config.announce_address_from_listening_address()
    }

    if let Some(wallet_address) = matches.value_of(WALLET_ADDRESS) {
        let trimmed = wallet_address.trim();
        validate_bech32_address_or_exit(trimmed);
        config = config.with_wallet_address(trimmed);
    }

    config
}

/// Ensures that a given bech32 address is valid, or exits
pub(crate) fn validate_bech32_address_or_exit(address: &str) {
    if let Err(bech32_address_validation::Bech32Error::DecodeFailed(err)) =
        bech32_address_validation::try_bech32_decode(address)
    {
        let error_message = format!("Error: wallet address decoding failed: {}", err).red();
        println!("{}", error_message);
        println!("Exiting...");
        process::exit(1);
    }

    if let Err(bech32_address_validation::Bech32Error::WrongPrefix(err)) =
        bech32_address_validation::validate_bech32_prefix(address)
    {
        let error_message = format!("Error: wallet address type is wrong, {}", err).red();
        println!("{}", error_message);
        println!("Exiting...");
        process::exit(1);
    }
}

// this only checks compatibility between config the binary. It does not take into consideration
// network version. It might do so in the future.
pub(crate) fn version_check(cfg: &Config) -> bool {
    let binary_version = env!("CARGO_PKG_VERSION");
    let config_version = cfg.get_version();
    if binary_version != config_version {
        warn!("The mixnode binary has different version than what is specified in config file! {} and {}", binary_version, config_version);
        if version_checker::is_minor_version_compatible(binary_version, config_version) {
            info!("but they are still semver compatible. However, consider running the `upgrade` command");
            true
        } else {
            error!("and they are semver incompatible! - please run the `upgrade` command before attempting `run` again");
            false
        }
    } else {
        true
    }
}
