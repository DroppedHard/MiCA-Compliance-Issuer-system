// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

import {AccessControl} from "@openzeppelin/contracts/access/AccessControl.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {ERC20Pausable} from "@openzeppelin/contracts/token/ERC20/extensions/ERC20Pausable.sol";

/// @title ResearchUsdEMT
/// @notice Local research token inspired by the operational core of Circle USDC.
/// @dev This demo is not issued money, is not backed by US dollars and makes no MiCA compliance claim.
contract ResearchUsdEMT is ERC20Pausable, AccessControl {
    enum TokenState { Active, Warning, IssuanceBlocked, WindDown }
    enum ReserveState { Active, Warning, IssuanceBlocked }
    bytes32 public constant MINTER_ROLE = keccak256("MINTER_ROLE");
    bytes32 public constant BURNER_ROLE = keccak256("BURNER_ROLE");
    bytes32 public constant PAUSER_ROLE = keccak256("PAUSER_ROLE");
    bytes32 public constant FREEZER_ROLE = keccak256("FREEZER_ROLE");
    bytes32 public constant WIND_DOWN_ROLE = keccak256("WIND_DOWN_ROLE");
    bytes32 public constant ISSUANCE_RESTRICTOR_ROLE = keccak256("ISSUANCE_RESTRICTOR_ROLE");

    mapping(address account => bool frozen) private _frozen;
    mapping(bytes32 operationId => bool processed) private _processedMintOperations;
    mapping(bytes32 operationId => bool processed) private _processedBurnOperations;
    bool public windDown;
    bool public issuanceBlocked;
    bytes32 public issuanceBlockEvidence;
    ReserveState public reserveState;
    bytes32 public reserveStateEvidence;

    error ZeroAddress();
    error AccountFrozen(address account);
    error MintOperationAlreadyProcessed(bytes32 operationId);
    error BurnOperationAlreadyProcessed(bytes32 operationId);
    error WindDownBlocksOperation();
    error IssuanceBlocked(bytes32 evidenceHash);
    error ReserveCoverageBlocksIssuance(bytes32 evidenceHash);

    event AddressFrozen(address indexed account);
    event AddressUnfrozen(address indexed account);
    event MintOperationExecuted(bytes32 indexed operationId, address indexed recipient, uint256 amount);
    event BurnOperationExecuted(bytes32 indexed operationId, address indexed holder, uint256 amount);
    event WindDownEntered(address indexed operator);
    event IssuanceBlockedByEvidence(address indexed operator, bytes32 indexed evidenceHash);
    event IssuanceUnblockedByEvidence(address indexed operator, bytes32 indexed evidenceHash);
    event ReserveStateChanged(
        address indexed operator,
        ReserveState previousState,
        ReserveState newState,
        bytes32 indexed evidenceHash
    );

    constructor(address admin) ERC20("Research USD EMT", "rUSD") {
        if (admin == address(0)) revert ZeroAddress();

        _grantRole(DEFAULT_ADMIN_ROLE, admin);
        _grantRole(MINTER_ROLE, admin);
        _grantRole(BURNER_ROLE, admin);
        _grantRole(PAUSER_ROLE, admin);
        _grantRole(FREEZER_ROLE, admin);
        _grantRole(WIND_DOWN_ROLE, admin);
        _grantRole(ISSUANCE_RESTRICTOR_ROLE, admin);
    }

    function decimals() public pure override returns (uint8) {
        return 6;
    }

    function mint(address to, uint256 amount) external onlyRole(MINTER_ROLE) {
        _requireIssuanceAllowed();
        _mint(to, amount);
    }

    /// @notice Executes one backend-correlated issuance operation exactly once.
    /// @dev The operation identifier protects retries across the issuer database,
    /// mock bank and blockchain boundary. It does not replace MINTER_ROLE access control.
    function mintForOperation(bytes32 operationId, address to, uint256 amount) external onlyRole(MINTER_ROLE) {
        _requireIssuanceAllowed();
        if (_processedMintOperations[operationId]) revert MintOperationAlreadyProcessed(operationId);
        _processedMintOperations[operationId] = true;
        _mint(to, amount);
        emit MintOperationExecuted(operationId, to, amount);
    }

    function isMintOperationProcessed(bytes32 operationId) external view returns (bool) {
        return _processedMintOperations[operationId];
    }

    /// @notice Executes one backend-correlated redemption burn exactly once.
    function burnForOperation(bytes32 operationId, address from, uint256 amount) external onlyRole(BURNER_ROLE) {
        if (_processedBurnOperations[operationId]) revert BurnOperationAlreadyProcessed(operationId);
        _processedBurnOperations[operationId] = true;
        _burn(from, amount);
        emit BurnOperationExecuted(operationId, from, amount);
    }

    function isBurnOperationProcessed(bytes32 operationId) external view returns (bool) {
        return _processedBurnOperations[operationId];
    }

    function burn(address from, uint256 amount) external onlyRole(BURNER_ROLE) {
        _burn(from, amount);
    }

    /// @notice Irreversibly enters orderly wind-down for this demo deployment.
    /// @dev Minting and ordinary transfers stop, while authorized burns required
    /// to settle holder redemptions remain possible. This is deliberately distinct
    /// from `pause`, which keeps its existing emergency-stop semantics and blocks
    /// every balance update, including burns.
    function enterWindDown() external onlyRole(WIND_DOWN_ROLE) {
        if (windDown) return;
        windDown = true;
        emit WindDownEntered(msg.sender);
    }

    /// @notice Blocks new issuance after enforceable activity-threshold evidence.
    /// @dev This is narrower than the emergency pause: transfers and redemption burns remain
    /// available. The evidence hash correlates the on-chain restriction with the persisted
    /// quarterly assessment held by the issuer backend.
    function blockIssuance(bytes32 evidenceHash) external onlyRole(ISSUANCE_RESTRICTOR_ROLE) {
        _setActivityIssuanceBlocked(true, evidenceHash);
    }

    /// @notice Restores issuance after a new authorised assessment no longer requires the block.
    /// @dev Wind-down and reserve restrictions remain independent and may still reject minting.
    function unblockIssuance(bytes32 evidenceHash) external onlyRole(ISSUANCE_RESTRICTOR_ROLE) {
        _setActivityIssuanceBlocked(false, evidenceHash);
    }

    function _setActivityIssuanceBlocked(bool blocked, bytes32 evidenceHash) private {
        if (issuanceBlocked == blocked && issuanceBlockEvidence == evidenceHash) return;
        issuanceBlocked = blocked;
        issuanceBlockEvidence = evidenceHash;
        if (blocked) {
            emit IssuanceBlockedByEvidence(msg.sender, evidenceHash);
        } else {
            emit IssuanceUnblockedByEvidence(msg.sender, evidenceHash);
        }
    }

    /// @notice Mirrors the latest issuer reserve assessment on-chain.
    /// @dev Like the activity-threshold control, this state is reversible:
    /// the issuer oracle may restore Warning or Active after reserve coverage recovers.
    function setReserveState(ReserveState newState, bytes32 evidenceHash)
        external
        onlyRole(ISSUANCE_RESTRICTOR_ROLE)
    {
        ReserveState previous = reserveState;
        if (previous == newState && reserveStateEvidence == evidenceHash) return;
        reserveState = newState;
        reserveStateEvidence = evidenceHash;
        emit ReserveStateChanged(msg.sender, previous, newState, evidenceHash);
    }

    /// @notice Returns the four-state lifecycle view used by the research system.
    /// @dev Wind-down takes precedence over the two reversible issuance restrictions.
    function tokenState() external view returns (TokenState) {
        if (windDown) return TokenState.WindDown;
        if (issuanceBlocked || reserveState == ReserveState.IssuanceBlocked) {
            return TokenState.IssuanceBlocked;
        }
        if (reserveState == ReserveState.Warning) return TokenState.Warning;
        return TokenState.Active;
    }

    /// @notice MiCA-oriented compliance control used to stop all token balance movement.
    /// @dev Required by this demo's compliance model so an authorized operator can
    /// execute incident-response or regulatory measures. MiCA defines issuer-level
    /// obligations and supervisory powers, rather than prescribing this Solidity API.
    function pause() external onlyRole(PAUSER_ROLE) {
        _pause();
    }

    /// @notice Resumes token movement after the authorized restriction is lifted.
    function unpause() external onlyRole(PAUSER_ROLE) {
        _unpause();
    }

    /// @notice MiCA-oriented compliance control for restricting one token holder.
    /// @dev Required by this demo's compliance model to implement sanctions, AML/CFT
    /// controls, access-denial policies, or a valid authority order. The restriction
    /// applies only to this token; it does not freeze the blockchain account itself.
    function freeze(address account) external onlyRole(FREEZER_ROLE) {
        if (account == address(0)) revert ZeroAddress();
        if (_frozen[account]) return;
        _frozen[account] = true;
        emit AddressFrozen(account);
    }

    /// @notice Removes an address restriction after an authorized decision.
    function unfreeze(address account) external onlyRole(FREEZER_ROLE) {
        if (account == address(0)) revert ZeroAddress();
        if (!_frozen[account]) return;
        _frozen[account] = false;
        emit AddressUnfrozen(account);
    }

    function isFrozen(address account) external view returns (bool) {
        return _frozen[account];
    }

    function _requireIssuanceAllowed() private view {
        if (issuanceBlocked) revert IssuanceBlocked(issuanceBlockEvidence);
        if (reserveState == ReserveState.IssuanceBlocked) {
            revert ReserveCoverageBlocksIssuance(reserveStateEvidence);
        }
    }

    function _update(address from, address to, uint256 value) internal override {
        if (from != address(0) && _frozen[from]) revert AccountFrozen(from);
        if (to != address(0) && _frozen[to]) revert AccountFrozen(to);
        if (windDown && to != address(0)) revert WindDownBlocksOperation();
        super._update(from, to, value);
    }
}
