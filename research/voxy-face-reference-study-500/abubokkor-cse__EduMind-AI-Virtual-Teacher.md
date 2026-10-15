# Facial rendering source scan: abubokkor-cse/EduMind-AI-Virtual-Teacher

Repository: https://github.com/abubokkor-cse/EduMind-AI-Virtual-Teacher

Pinned source revision: 73781e968856caa67d28c3251b318e43e90dce56

Discovery topic: lip sync 3D

Description: AI Virtual Teacher with real-time lip-synced 3D avatars powered by Gemini Live API — voice conversation, image generation, smart quizzes, and deep research

Reported license: NOASSERTION

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"face": ["lib/three/examples/jsm/curves/NURBSSurface.js", "lib/three/examples/jsm/libs/surfaceNet.js"], "lip": ["modules/lipsync-de.mjs", "modules/lipsync-en.mjs", "modules/lipsync-fi.mjs", "modules/lipsync-fr.mjs", "modules/lipsync-lt.mjs"]}

## Source file: lib/three/examples/jsm/libs/surfaceNet.js

{
  "path": "lib/three/examples/jsm/libs/surfaceNet.js",
  "blob_sha": "527c3eef6eeaa797d95552746f11aea68be1fc5f",
  "sha256": "a3b63a8038ed712471310583d91ba8c0afb29dfd99ab80e41afca354f69ed4ff",
  "lines": 201,
  "bytes": 5545,
  "selection_score": 13,
  "keyword_line_counts": {
    "face": 12,
    "lip": 1
  },
  "url": "https://github.com/abubokkor-cse/EduMind-AI-Virtual-Teacher/blob/73781e968856caa67d28c3251b318e43e90dce56/lib/three/examples/jsm/libs/surfaceNet.js",
  "function_signature_candidates": []
}

```text

/**
 * SurfaceNets in JavaScript
 *
 * Written by Mikola Lysenko (C) 2012
 *
 * MIT License
 *
 * Based on: S.F. Gibson, 'Constrained Elastic Surface Nets'. (1998) MERL Tech Report.
 * from https://github.com/mikolalysenko/isosurface/tree/master
 * 
 */

let surfaceNet = ( dims, potential, bounds ) => {
		
	
	//Precompute edge table, like Paul Bourke does.
	// This saves a bit of time when computing the centroid of each boundary cell
	var cube_edges = new Int32Array(24) , edge_table = new Int32Array(256);
	(function() {

		//Initialize the cube_edges table
		// This is just the vertex number of each cube
		var k = 0;
		for(var i=0; i<8; ++i) {
			for(var j=1; j<=4; j<<=1) {
				var p = i^j;
				if(i <= p) {
					cube_edges[k++] = i;
					cube_edges[k++] = p;
				}
			}
		}

		//Initialize the intersection table.
		//  This is a 2^(cube configuration) ->  2^(edge configuration) map
		//  There is one entry for each possible cube configuration, and the output is a 12-bit vector enumerating all edges crossing the 0-level.
		for(var i=0; i<256; ++i) {
			var em = 0;
			for(var j=0; j<24; j+=2) {
				var a = !!(i & (1<<cube_edges[j]))
					, b = !!(i & (1<<cube_edges[j+1]));
				em |= a !== b ? (1 << (j >> 1)) : 0;
			}
			edge_table[i] = em;
		}
	})();

	//Internal buffer, this may get resized at run time
	var buffer = new Array(4096);
	(function() {
		for(var i=0; i<buffer.length; ++i) {
			buffer[i] = 0;
		}
	})();

	if(!bounds) {
		bounds = [[0,0,0],dims];
	}
	
	var scale     = [0,0,0];
	var shift     = [0,0,0];
	for(var i=0; i<3; ++i) {
		scale[i] = (bounds[1][i] - bounds[0][i]) / dims[i];
		shift[i] = bounds[0][i];
	}
	
	var vertices = []
		, faces = []
		, n = 0
		, x = [0, 0, 0]
		, R = [1, (dims[0]+1), (dims[0]+1)*(dims[1]+1)]
		, grid = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
		, buf_no = 1;
	
		
	//Resize buffer if necessary 
	if(R[2] * 2 > buffer.length) {
		var ol = buffer.length;
		buffer.length = R[2] * 2;
		while(ol < buffer.length) {
			buffer[ol++] = 0;
		}
	}
	
	//March over the voxel grid
	for(x[2]=0; x[2]<dims[2]-1; ++x[2], n+=dims[0], buf_no ^= 1, R[2]=-R[2]) {
	
		//m is the pointer into the buffer we are going to use.  
		//This is slightly obtuse because javascript does not have good support for packed data structures, so we must use typed arrays :(
		//The contents of the buffer will be the indices of the vertices on the previous x/y slice of the volume
		var m = 1 + (dims[0]+1) * (1 + buf_no * (dims[1]+1));
		
		for(x[1]=0; x[1]<dims[1]-1; ++x[1], ++n, m+=2)
		for(x[0]=0; x[0]<dims[0]-1; ++x[0], ++n, ++m) {
		
			//Read in 8 field values around this vertex and store them in an array
			//Also calculate 8-bit mask, like in marching cubes, so we can speed up sign checks later
			var mask = 0, g = 0;
			for(var k=0; k<2; ++k)
			for(var j=0; j<2; ++j)      
			for(var i=0; i<2; ++i, ++g) {
				var p = potential(
					scale[0]*(x[0]+i)+shift[0],
					scale[1]*(x[1]+j)+shift[1],
					scale[2]*(x[2]+k)+shift[2]);
				grid[g] = p;
				mask |= (p < 0) ? (1<<g) : 0;
			}
			
			//Check for early termination if cell does not intersect boundary
			if(mask === 0 || mask === 0xff) {
				continue;
			}
			
			//Sum up edge intersections
			var edge_mask = edge_table[mask]
				, v = [0.0,0.0,0.0]
				, e_count = 0;
				
			//For every edge of the cube...
			for(var i=0; i<12; ++i) {
			
				//Use edge mask to check if it is crossed
				if(!(edge_mask & (1<<i))) {
					continue;
				}
				
				//If it did, increment number of edge crossings
				++e_count;
				
				//Now find the point of intersection
				var e0 = cube_edges[ i<<1 ]       //Unpack vertices
					, e1 = cube_edges[(i<<1)+1]
					, g0 = grid[e0]                 //Unpack grid values
					, g1 = grid[e1]
					, t  = g0 - g1;                 //Compute point of intersection
				if(Math.abs(t) > 1e-6) {
					t = g0 / t;
				} else {
					continue;
				}
				
				//Interpolate vertices and add up intersections (this can be done without multiplying)
				for(var j=0, k=1; j<3; ++j, k<<=1) {
					var a = e0 & k
						, b = e1 & k;
					if(a !== b) {
						v[j] += a ? 1.0 - t : t;
					} else {
						v[j] += a ? 1.0 : 0;
					}
				}
			}
			
			//Now we just average the edge intersections and add them to coordinate
			var s = 1.0 / e_count;
			for(var i=0; i<3; ++i) {
				v[i] = scale[i] * (x[i] + s * v[i]) + shift[i];
			}
			
			//Add vertex to buffer, store pointer to vertex index in buffer
			buffer[m] = vertices.length;
			vertices.push(v);
			
			//Now we need to add faces together, to do this we just loop over 3 basis components
			for(var i=0; i<3; ++i) {
				//The first three entries of the edge_mask count the crossings along the edge
				if(!(edge_mask & (1<<i)) ) {
					continue;
				}
				
				// i = axes we are point along.  iu, iv = orthogonal axes
				var iu = (i+1)%3
					, iv = (i+2)%3;
					
				//If we are on a boundary, skip it
				if(x[iu] === 0 || x[iv] === 0) {
					continue;
				}
				
				//Otherwise, look up adjacent edges in buffer
				var du = R[iu]
					, dv = R[iv];
				
				//Remember to flip orientation depending on the sign of the corner.
				if(mask & 1) {
					faces.push([buffer[m],    buffer[m-du],    buffer[m-dv]]);
					faces.push([buffer[m-dv], buffer[m-du],    buffer[m-du-dv]]);
				} else {
					faces.push([buffer[m],    buffer[m-dv],    buffer[m-du]]);
					faces.push([buffer[m-du], buffer[m-dv],    buffer[m-du-dv]]);
				}
			}
		}
	}
	
	//All done!  Return the result
	return { positions: vertices, cells: faces };
}

export { surfaceNet }

```

## Source file: lib/three/examples/jsm/curves/NURBSSurface.js

{
  "path": "lib/three/examples/jsm/curves/NURBSSurface.js",
  "blob_sha": "67da9f2bff77174fe077b521586d9df6a11101a1",
  "sha256": "a7ff303addd765da8c85ceabb2e3248a970144021cd52b2cb8f177e34aeaa244",
  "lines": 98,
  "bytes": 2687,
  "selection_score": 13,
  "keyword_line_counts": {
    "face": 9
  },
  "url": "https://github.com/abubokkor-cse/EduMind-AI-Virtual-Teacher/blob/73781e968856caa67d28c3251b318e43e90dce56/lib/three/examples/jsm/curves/NURBSSurface.js",
  "function_signature_candidates": []
}

```text

import {
	Vector4
} from 'three';
import * as NURBSUtils from '../curves/NURBSUtils.js';

/**
 * This class represents a NURBS surface.
 *
 * Implementation is based on `(x, y [, z=0 [, w=1]])` control points with `w=weight`.
 *
 * @three_import import { NURBSSurface } from 'three/addons/curves/NURBSSurface.js';
 */
class NURBSSurface {

	/**
	 * Constructs a new NURBS surface.
	 *
	 * @param {number} degree1 - The first NURBS degree.
	 * @param {number} degree2 - The second NURBS degree.
	 * @param {Array<number>} knots1 - The first knots as a flat array of numbers.
	 * @param {Array<number>} knots2 - The second knots as a flat array of numbers.
	 * @param {Array<Array<Vector2|Vector3|Vector4>>} controlPoints - An array^2 holding control points.
	 */
	constructor( degree1, degree2, knots1, knots2, controlPoints ) {

		/**
		 * The first NURBS degree.
		 *
		 * @type {number}
		 */
		this.degree1 = degree1;

		/**
		 * The second NURBS degree.
		 *
		 * @type {number}
		 */
		this.degree2 = degree2;

		/**
		 * The first knots as a flat array of numbers.
		 *
		 * @type {Array<number>}
		 */
		this.knots1 = knots1;

		/**
		 * The second knots as a flat array of numbers.
		 *
		 * @type {Array<number>}
		 */
		this.knots2 = knots2;

		/**
		 *  An array holding arrays of control points.
		 *
		 * @type {Array<Array<Vector2|Vector3|Vector4>>}
		 */
		this.controlPoints = [];

		const len1 = knots1.length - degree1 - 1;
		const len2 = knots2.length - degree2 - 1;

		// ensure Vector4 for control points
		for ( let i = 0; i < len1; ++ i ) {

			this.controlPoints[ i ] = [];

			for ( let j = 0; j < len2; ++ j ) {

				const point = controlPoints[ i ][ j ];
				this.controlPoints[ i ][ j ] = new Vector4( point.x, point.y, point.z, point.w );

			}

		}

	}

	/**
	 * This method returns a vector in 3D space for the given interpolation factor. This vector lies on the NURBS surface.
	 *
	 * @param {number} t1 - The first interpolation factor representing the `u` position on the surface. Must be in the range `[0,1]`.
	 * @param {number} t2 - The second interpolation factor representing the `v` position on the surface. Must be in the range `[0,1]`.
	 * @param {Vector3} target - The target vector the result is written to.
	 */
	getPoint( t1, t2, target ) {

		const u = this.knots1[ 0 ] + t1 * ( this.knots1[ this.knots1.length - 1 ] - this.knots1[ 0 ] ); // linear mapping t1->u
		const v = this.knots2[ 0 ] + t2 * ( this.knots2[ this.knots2.length - 1 ] - this.knots2[ 0 ] ); // linear mapping t2->u

		NURBSUtils.calcSurfacePoint( this.degree1, this.degree2, this.knots1, this.knots2, this.controlPoints, u, v, target );

	}

}

export { NURBSSurface };


```

## Source file: auth.js

{
  "path": "auth.js",
  "blob_sha": "4b2e56fc179f9830a1fb5e8fd8a3e60f58093334",
  "sha256": "e6265cc5383775f95c7d0e9635bed2ff6662a35d78a8c0d34f16e69625b9635e",
  "lines": 1391,
  "bytes": 49675,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1
  },
  "url": "https://github.com/abubokkor-cse/EduMind-AI-Virtual-Teacher/blob/73781e968856caa67d28c3251b318e43e90dce56/auth.js",
  "function_signature_candidates": []
}

```text

// Authentication & Data Management Module for EduMind
// Handles user auth, profile, progress, chat history

import {
    auth,
    db,
    storage,
    signInWithEmailAndPassword,
    createUserWithEmailAndPassword,
    signInWithPopup,
    googleProvider,
    onAuthStateChanged,
    signOut,
    sendPasswordResetEmail,
    updateProfile,
    doc,
    setDoc,
    getDoc,
    updateDoc,
    deleteDoc,
    collection,
    addDoc,
    query,
    where,
    orderBy,
    limit,
    getDocs,
    ref,
    uploadBytes,
    getDownloadURL
} from './firebase-config.js';

// Current user state
let currentUser = null;
let userProfile = null;

// ===========================================
// Cached Auth State (for instant UI)
// ===========================================

// Cache keys
const AUTH_CACHE_KEY = 'edumind_auth_cache';

// Cache auth state to localStorage for instant UI
function cacheAuthState(user) {
    if (user) {
        const cachedData = {
            uid: user.uid,
            email: user.email,
            displayName: user.displayName,
            photoURL: user.photoURL,
            isLoggedIn: true,
            cachedAt: Date.now()
        };
        localStorage.setItem(AUTH_CACHE_KEY, JSON.stringify(cachedData));
    } else {
        localStorage.removeItem(AUTH_CACHE_KEY);
    }
}

// Get cached auth state
export function getCachedAuthState() {
    try {
        const cached = localStorage.getItem(AUTH_CACHE_KEY);
        if (cached) {
            const data = JSON.parse(cached);
            // Cache is valid for 7 days
            if (Date.now() - data.cachedAt < 7 * 24 * 60 * 60 * 1000) {
                return data;
            }
        }
    } catch (e) {
        console.error("Error reading auth cache:", e);
    }
    return null;
}

// Update header UI based on auth state (instant, no waiting)
export function updateHeaderAuthUI(authState) {
    const userBtn = document.getElementById('user-btn');
    const userBtnText = document.getElementById('user-btn-text');
    const userBtnIcon = document.getElementById('user-btn-icon');
    const userBtnAvatar = document.getElementById('user-btn-avatar');

    if (authState && authState.isLoggedIn) {
        // User is logged in - show profile picture
        if (userBtn) userBtn.classList.add('logged-in');
        if (userBtnText) userBtnText.textContent = '';

        if (authState.photoURL && userBtnAvatar) {
            userBtnAvatar.src = authState.photoURL;
            userBtnAvatar.classList.remove('hidden');
            if (userBtnIcon) userBtnIcon.classList.add('hidden');
        } else {
            // Show first letter of name or email as avatar
            if (userBtnIcon) {
                const initial = (authState.displayName || authState.email || '').charAt(0).toUpperCase() || '👤';
                userBtnIcon.textContent = initial;
                userBtnIcon.classList.remove('hidden');
                userBtnIcon.style.fontSize = '1rem';
                userBtnIcon.style.fontWeight = '600';
            }
            if (userBtnAvatar) userBtnAvatar.classList.add('hidden');
        }
    } else {
        // User not logged in - show login
        if (userBtn) userBtn.classList.remove('logged-in');
        if (userBtnText) userBtnText.textContent = 'Login';
        if (userBtnIcon) {
            userBtnIcon.textContent = '👤';
            userBtnIcon.classList.remove('hidden');
            userBtnIcon.style.fontSize = '';
            userBtnIcon.style.fontWeight = '';
        }
        if (userBtnAvatar) userBtnAvatar.classList.add('hidden');
    }
}

// ===========================================
// Authentication Functions
// ===========================================

// Initialize auth state listener
export function initAuth() {
    // First, instantly update UI from cache (no loading delay)
    const cachedAuth = getCachedAuthState();
    if (cachedAuth) {
        console.log("⚡ Using cached auth state for instant UI");
        updateHeaderAuthUI(cachedAuth);
    }

    return new Promise((resolve) => {
        const unsubscribe = onAuthStateChanged(auth, async (user) => {
            if (user) {
                console.log("✅ User signed in:", user.email);
                currentUser = user;

                // Cache the auth state for next page load
                cacheAuthState(user);

                // Update header UI with real data
                updateHeaderAuthUI({
                    isLoggedIn: true,
                    displayName: user.displayName,
                    email: user.email,
                    photoURL: user.photoURL
                });

                await loadUserProfile(user.uid);

                // Update credits and BKT progress display
                if (typeof window.updateCreditsDisplay === 'function') {
                    await window.updateCreditsDisplay();
                }
                // Don't show profile UI here - let app.js handle it after student profile loads
            } else {
                console.log("❌ No user signed in");
                currentUser = null;
                userProfile = null;

                // Clear cache and update UI
                cacheAuthState(null);
                updateHeaderAuthUI(null);

                // Update credits display to show 0 for guest
                if (typeof window.updateCreditsDisplay === 'function') {
                    await window.updateCreditsDisplay();
                }
            }
            // Resolve the promise after first auth state check
            resolve(user);
        });
    });
}

// Email/Password Login
export async function loginWithEmail(email, password) {
    try {
        const userCredential = await signInWithEmailAndPassword(auth, email, password);
        console.log("✅ Login successful");
        return { success: true, user: userCredential.user };
    } catch (error) {
        console.error("❌ Login error:", error);
        return { success: false, error: error.message };
    }
}

// Email/Password Signup
export async function signupWithEmail(email, password, displayName, grade) {
    try {
        const userCredential = await createUserWithEmailAndPassword(auth, email, password);
        const user = userCredential.user;

        // Create user profile
        await createUserProfile(user.uid, {
            email: user.email,
            displayName: displayName,
            grade: parseInt(grade),
            createdAt: new Date(),
            lastActive: new Date(),
            totalPoints: 0,
            streak: 0,
            subjects: []
        });

        console.log("✅ Signup successful");
        return { success: true, user };
    } catch (error) {
        console.error("❌ Signup error:", error);
        return { success: false, error: error.message };
    }
}

// Google Sign-in
export async function loginWithGoogle() {
    try {
        const result = await signInWithPopup(auth, googleProvider);
        const user = result.user;

        // Check if profile exists, if not create one
        const profileExists = await getDoc(doc(db, "users", user.uid));
        if (!profileExists.exists()) {
            await createUserProfile(user.uid, {
                email: user.email,
                displayName: user.displayName || "Student",
                grade: 10, // Default grade
                createdAt: new Date(),
                lastActive: new Date(),
                totalPoints: 0,
                streak: 0,
                subjects: []
            });
        }

        console.log("✅ Google login successful");
        return { success: true, user };
    } catch (error) {
        console.error("❌ Google login error:", error);
        return { success: false, error: error.message };
    }
}

// Logout
export async function logout() {
    try {
        await signOut(auth);
        currentUser = null;
        userProfile = null;
        console.log("✅ Logout successful");
        return { success: true };
    } catch (error) {
        console.error("❌ Logout error:", error);
        return { success: false, error: error.message };
    }
}

// Password Reset
export async function resetPassword(email) {
    try {
        await sendPasswordResetEmail(auth, email);
        console.log("✅ Password reset email sent");
        return { success: true, message: "Password reset email sent! Check your inbox." };
    } catch (error) {
        console.error("❌ Password reset error:", error);
        return { success: false, error: error.message };
    }
}

// Upload Profile Picture
export async function uploadProfilePicture(file) {
    if (!currentUser) return { success: false, error: "Not logged in" };

    try {
        // Create storage reference
        const storageRef = ref(storage, `profile-pictures/${currentUser.uid}`);

        // Upload file
        await uploadBytes(storageRef, file);

        // Get download URL
        const photoURL = await getDownloadURL(storageRef);

        // Update auth profile
        await updateProfile(currentUser, { photoURL });

        // Update Firestore profile
        await updateDoc(doc(db, "users", currentUser.uid), { photoURL });

        console.log("✅ Profile picture uploaded");
        return { success: true, photoURL };
    } catch (error) {
        console.error("❌ Profile picture upload error:", error);
        return { success: false, error: error.message };
    }
}

// Update Display Name
export async function updateDisplayName(displayName) {
    if (!currentUser) return { success: false, error: "Not logged in" };

    try {
        await updateProfile(currentUser, { displayName });
        await updateDoc(doc(db, "users", currentUser.uid), { displayName });
        console.log("✅ Display name updated");
        return { success: true };
    } catch (error) {
        console.error("❌ Display name update error:", error);
        return { success: false, error: error.message };
    }
}

// ===========================================
// User Profile Functions
// ===========================================

// Create user profile
async function createUserProfile(userId, profileData) {
    await setDoc(doc(db, "users", userId), profileData);
}

// Load user profile
async function loadUserProfile(userId) {
    try {
        const docSnap = await getDoc(doc(db, "users", userId));
        if (docSnap.exists()) {
            userProfile = { id: userId, ...docSnap.data() };

            // Update last active
            await updateDoc(doc(db, "users", userId), {
                lastActive: new Date()
            });

            return userProfile;
        }
    } catch (error) {
        console.error("❌ Error loading profile:", error);
    }
    return null;
}

// Get current user
export function getCurrentUser() {
    return currentUser;
}

// Get user profile
export function getUserProfile() {
    return userProfile;
}

// Update user profile
export async function updateUserProfile(updates) {
    if (!currentUser) return { success: false, error: "Not logged in" };

    try {
        await updateDoc(doc(db, "users", currentUser.uid), updates);
        userProfile = { ...userProfile, ...updates };
        console.log("✅ Profile updated");
        return { success: true };
    } catch (error) {
        console.error("❌ Profile update error:", error);
        return { success: false, error: error.message };
    }
}

// ===========================================
// Progress Tracking Functions
// ===========================================

// Save topic progress (BKT knowledge level)
export async function saveProgress(subject, topic, knowledgeLevel, attempts) {
    if (!currentUser) return { success: false, error: "Not logged in" };

    // Validate inputs to prevent undefined values
    if (!subject || !topic) {
        console.warn("⚠️ Invalid progress data: missing subject or topic");
        return { success: false, error: "Missing subject or topic" };
    }

    // Ensure values are not undefined
    const safeKnowledgeLevel = knowledgeLevel ?? 0;
    const safeAttempts = attempts ?? 0;

    // Sanitize subject and topic for document ID (remove special characters)
    const safeSubject = String(subject).replace(/[\/\.\#\$\[\]]/g, '_');
    const safeTopic = String(topic).replace(/[\/\.\#\$\[\]]/g, '_');
    const progressId = `${currentUser.uid}_${safeSubject}_${safeTopic}`;

    try {
        await setDoc(doc(db, "progress", progressId), {
            userId: currentUser.uid,
            subject: safeSubject,
            topic: safeTopic,
            knowledgeLevel: safeKnowledgeLevel,
            attempts: safeAttempts,
            lastPracticed: new Date(),
            updatedAt: new Date()
        });

        console.log("✅ Progress saved:", safeSubject, safeTopic);
        return { success: true };
    } catch (error) {
        console.error("❌ Progress save error:", error);
        return { success: false, error: error.message };
    }
}

// Save overall learning state (achievements, streaks, points, level)
export async function saveLearningState(progressTracker) {
    if (!currentUser) return { success: false, error: "Not logged in" };

    try {
        // Sanitize data to remove undefined values
        const cleanData = {
            userId: currentUser.uid,
            totalPoints: progressTracker.totalPoints ?? 0,
            level: progressTracker.level ?? 1,
            streakData: {
                currentStreak: progressTracker.streakData?.currentStreak ?? 0,
                longestStreak: progressTracker.streakData?.longestStreak ?? 0,
                lastActiveDate: progressTracker.streakData?.lastActiveDate ?? new Date().toISOString().split('T')[0]
            },
            achievements: Array.isArray(progressTracker.achievements)
                ? progressTracker.achievements.filter(a => a !== undefined)
                : [],
            updatedAt: new Date()
        };

        await setDoc(doc(db, "learningState", currentUser.uid), cleanData);
        console.log("✅ Learning state saved");
        return { success: true };
    } catch (error) {
        console.error("❌ Learning state save error:", error);
        return { success: false, error: error.message };
    }
}

// Load learning state
export async function loadLearningState() {
    if (!currentUser) return null;

    try {
        const docSnap = await getDoc(doc(db, "learningState", currentUser.uid));
        if (docSnap.exists()) {
            console.log("✅ Learning state loaded");
            return docSnap.data();
        }
        return null;
    } catch (error) {
        console.error("❌ Learning state load error:", error);
        return null;
    }
}

// Load all progress for user
export async function loadProgress() {
    if (!currentUser) return [];

    try {
        const q = query(
            collection(db, "progress"),
            where("userId", "==", currentUser.uid)
        );
        const querySnapshot = await getDocs(q);

        const progressData = [];
        querySnapshot.forEach((doc) => {
            progressData.push({ id: doc.id, ...doc.data() });
        });

        console.log("✅ Loaded", progressData.length, "progress records");
        return progressData;
    } catch (error) {
        console.error("❌ Progress load error:", error);
        return [];
    }
}

// ===========================================
// Chat History Functions (Professional like Claude/ChatGPT)
// ===========================================

// Current active chat session ID
let currentChatId = null;

// Get current chat ID
export function getCurrentChatId() {
    return currentChatId;
}

// Set current chat ID
export function setCurrentChatId(chatId) {
    currentChatId = chatId;
}

// Generate smart chat title from conversation
function generateChatTitle(messages) {
    if (!messages || messages.length === 0) {
        console.log("📝 Title: No messages");
        return 'New Chat';
    }

    // Find first user message
    const firstUserMsg = messages.find(msg => msg.role === 'user');
    if (!firstUserMsg) {
        console.log("📝 Title: No user message found");
        return 'New Chat';
    }

    // Get text from message - check multiple formats
    let text = '';
    if (firstUserMsg.content && typeof firstUserMsg.content === 'string') {
        text = firstUserMsg.content;
    } else if (firstUserMsg.parts && Array.isArray(firstUserMsg.parts)) {
        text = firstUserMsg.parts.map(p => p.text || '').join(' ');
    } else if (firstUserMsg.text) {
        text = firstUserMsg.text;
    }

    console.log("📝 Title source text:", text.substring(0, 100));

    // Clean text
    text = text.trim().replace(/\n/g, ' ').replace(/\s+/g, ' ');

    if (!text) {
        console.log("📝 Title: Empty text after cleaning");
        return 'New Chat';
    }

    // Smart title extraction - try to get the main topic/question
    // Remove common question starters to get the topic
    let title = text
        .replace(/^(hi|hello|hey|please|can you|could you|would you|will you|i want to|i need to|help me|teach me|explain|tell me about|tell me|what is|what are|what's|how to|how do|how does|why is|why are|why do|when is|when are|where is|where are|who is|who are|i have a question about|question about)\s*/gi, '')
        .trim();

    // If title became empty after removing starters, use original text
    if (!title) {
        title = text;
    }

    // Capitalize first letter
    if (title.length > 0) {
        title = title.charAt(0).toUpperCase() + title.slice(1);
    }

    // Truncate if too long
    if (title.length > 50) {
        // Try to cut at a word boundary
        const cutoff = title.lastIndexOf(' ', 47);
        title = title.substring(0, cutoff > 20 ? cutoff : 47) + '...';
    }

    console.log("📝 Generated title:", title);
    return title || 'New Chat';
}

// Save new chat session
export async function saveChatHistory(mode, messages) {
    if (!currentUser) return { success: false, error: "Not logged in" };

    // Validate and clean messages
    if (!Array.isArray(messages) || messages.length === 0) {
        console.warn("⚠️ No valid messages to save");
        return { success: false, error: "No messages to save" };
    }

    // Require at least 2 messages (1 user + 1 reply) for a real conversation
    const hasUserMsg = messages.some(m => m.role === 'user');
    const hasModelMsg = messages.some(m => m.role === 'model');
    if (!hasUserMsg || !hasModelMsg) {
        console.warn("⚠️ Need both user and model messages to save");
        return { success: false, error: "Need conversation to save" };
    }

    // Filter and sanitize messages (keep image, quiz, and other data)
    const cleanedMessages = messages
        .filter(msg => msg && (msg.role || msg.parts || msg.content))
        .map(msg => ({
            role: msg.role || 'user',
            content: msg.content || '',
            type: msg.type || 'text', // 'text', 'image', 'file', 'quiz', 'quiz_result'
            imageData: msg.imageData || null, // base64 image data
            mimeType: msg.mimeType || null, // image mime type
            fileName: msg.fileName || null, // uploaded file name
            quizData: msg.quizData || null, // quiz questions data
            quizResult: msg.quizResult || null, // quiz result data
            parts: Array.isArray(msg.parts)
                ? msg.parts.map(part => ({
                    text: part.text ?? ''
                }))
                : [{ text: msg.content || '' }],
            timestamp: msg.timestamp || new Date().toISOString()
        }));

    if (cleanedMessages.length === 0) {
        console.warn("⚠️ No valid messages after cleaning");
        return { success: false, error: "No valid messages" };
    }

    try {
        const title = generateChatTitle(cleanedMessages);
        const chatRef = await addDoc(collection(db, "chatHistory"), {
            userId: currentUser.uid,
            title: title,
            mode: mode || 'chat',
            messages: cleanedMessages,
            messageCount: cleanedMessages.length,
            createdAt: new Date(),
            updatedAt: new Date()
        });

        currentChatId = chatRef.id;
        console.log("✅ Chat history saved:", chatRef.id, "Title:", title);
        return { success: true, chatId: chatRef.id, title: title };
    } catch (error) {
        console.error("❌ Chat save error:", error);
        return { success: false, error: error.message };
    }
}

// Update existing chat session (add new messages)
export async function updateChatHistory(chatId, messages) {
    if (!currentUser) return { success: false, error: "Not logged in" };
    if (!chatId) return { success: false, error: "No chat ID" };

    // Clean messages (keep image, quiz, and other data)
    const cleanedMessages = messages
        .filter(msg => msg && (msg.role || msg.parts || msg.content))
        .map(msg => ({
            role: msg.role || 'user',
            content: msg.content || '',
            type: msg.type || 'text',
            imageData: msg.imageData || null,
            mimeType: msg.mimeType || null,
            fileName: msg.fileName || null,
            quizData: msg.quizData || null,
            quizResult: msg.quizResult || null,
            parts: Array.isArray(msg.parts)
                ? msg.parts.map(part => ({
                    text: part.text ?? ''
                }))
                : [{ text: msg.content || '' }],
            timestamp: msg.timestamp || new Date().toISOString()
        }));

    try {
        const chatRef = doc(db, "chatHistory", chatId);
        await updateDoc(chatRef, {
            messages: cleanedMessages,
            messageCount: cleanedMessages.length,
            updatedAt: new Date()
        });

        console.log("✅ Chat updated:", chatId);
        return { success: true, chatId: chatId };
    } catch (error) {
        console.error("❌ Chat update error:", error);
        return { success: false, error: error.message };
    }
}

// Load recent chat history list (for sidebar) - with offline cache
export async function loadChatHistory(limitCount = 50) {
    const CACHE_KEY = 'edumind_chat_history_cache';

    // Try to load from cache first (for immediate display)
    let cachedChats = [];
    try {
        const cached = localStorage.getItem(CACHE_KEY);
        if (cached) {
            cachedChats = JSON.parse(cached);
            console.log("📦 Loaded", cachedChats.length, "chats from cache");
        }
    } catch (e) {
        console.warn("Cache read error:", e);
    }

    // If not logged in, return cached data
    if (!currentUser) {
        console.log("👤 No user, returning cached chats");
        return cachedChats;
    }

    // Check if online
    if (!navigator.onLine) {
        console.log("📴 Offline - using cached chat history");
        return cachedChats;
    }

    try {
        // Query chats ordered by createdAt (no composite index needed)
        // We filter by userId and sort by creation time
        const q = query(
            collection(db, "chatHistory"),
            where("userId", "==", currentUser.uid),
            orderBy("createdAt", "desc"),
            limit(limitCount)
        );
        const querySnapshot = await getDocs(q);

        const chats = [];
        querySnapshot.forEach((doc) => {
            const data = doc.data();
            chats.push({
                id: doc.id,
                title: data.title || 'Untitled Chat',
                mode: data.mode || 'chat',
                messageCount: data.messageCount || data.messages?.length || 0,
                createdAt: data.createdAt?.toDate?.() || new Date(data.createdAt),
                updatedAt: data.updatedAt?.toDate?.() || new Date(data.updatedAt)
            });
        });

        console.log("✅ Loaded", chats.length, "chat sessions from Firebase");

        // Cache to localStorage for offline access
        try {
            localStorage.setItem(CACHE_KEY, JSON.stringify(chats));
            console.log("💾 Chat history cached for offline");
        } catch (e) {
            console.warn("Cache write error:", e);
        }

        return chats;
    } catch (error) {
        console.error("❌ Chat load error:", error);
        // Return cached data on error
        console.log("📦 Returning cached chats due to error");
        return cachedChats;
    }
}

// Load single chat with full messages - with offline cache
export async function loadChat(chatId) {
    const CACHE_KEY = `edumind_chat_${chatId}`;

    // Try to load from cache first
    let cachedChat = null;
    try {
        const cached = localStorage.getItem(CACHE_KEY);
        if (cached) {
            cachedChat = JSON.parse(cached);
            console.log("📦 Loaded chat from cache:", chatId);
        }
    } catch (e) {
        console.warn("Cache read error:", e);
    }

    if (!currentUser) {
        return cachedChat;
    }
    if (!chatId) return null;

    // Check if online
    if (!navigator.onLine) {
        console.log("📴 Offline - using cached chat");
        return cachedChat;
    }

    try {
        const chatRef = doc(db, "chatHistory", chatId);
        const chatSnap = await getDoc(chatRef);

        if (chatSnap.exists()) {
            const data = chatSnap.data();

            // Verify ownership
            if (data.userId !== currentUser.uid) {
                console.error("❌ Unauthorized access to chat");
                return null;
            }

            const chat = {
                id: chatSnap.id,
                ...data,
                createdAt: data.createdAt?.toDate?.() || new Date(data.createdAt),
                updatedAt: data.updatedAt?.toDate?.() || new Date(data.updatedAt)
            };

            console.log("✅ Loaded chat from Firebase:", chatId);

            // Cache for offline access
            try {
                localStorage.setItem(CACHE_KEY, JSON.stringify(chat));
                console.log("💾 Chat cached for offline:", chatId);
            } catch (e) {
                console.warn("Cache write error:", e);
            }

            return chat;
        } else {
            console.warn("⚠️ Chat not found:", chatId);
            return cachedChat; // Return cached if available
        }
    } catch (error) {
        console.error("❌ Chat load error:", error);
        // Return cached data on error
        return cachedChat;
    }
}

// Delete chat
export async function deleteChat(chatId) {
    if (!currentUser) return { success: false, error: "Not logged in" };
    if (!chatId) return { success: false, error: "No chat ID" };

    try {
        // First verify ownership
        const chatRef = doc(db, "chatHistory", chatId);
        const chatSnap = await getDoc(chatRef);

        if (!chatSnap.exists()) {
            return { success: false, error: "Chat not found" };
        }

        if (chatSnap.data().userId !== currentUser.uid) {
            return { success: false, error: "Unauthorized" };
        }

        // Delete using imported deleteDoc
        await deleteDoc(chatRef);

        // Clear current chat if it was deleted
        if (currentChatId === chatId) {
            currentChatId = null;
        }

        console.log("✅ Chat deleted:", chatId);
        return { success: true };
    } catch (error) {
        console.error("❌ Chat delete error:", error);
        return { success: false, error: error.message };
    }
}

// Rename chat
export async function renameChat(chatId, newTitle) {
    if (!currentUser) return { success: false, error: "Not logged in" };
    if (!chatId) return { success: false, error: "No chat ID" };

    try {
        const chatRef = doc(db, "chatHistory", chatId);
        await updateDoc(chatRef, {
            title: newTitle,
            updatedAt: new Date()
        });

        console.log("✅ Chat renamed:", chatId, "to", newTitle);
        return { success: true };
    } catch (error) {
        console.error("❌ Chat rename error:", error);
        return { success: false, error: error.message };
    }
}

// ===========================================
// Quiz Results Functions
// ===========================================

// Save quiz result
export async function saveQuizResult(subject, topic, score, totalQuestions, timeTaken, answers) {
    if (!currentUser) return { success: false, error: "Not logged in" };

    try {
        await addDoc(collection(db, "quizResults"), {
            userId: currentUser.uid,
            subject,
            topic,
            score,
            totalQuestions,
            timeTaken,
            answers,
            completedAt: new Date()
        });

        // Update user points
        const newPoints = userProfile.totalPoints + score;
        await updateUserProfile({ totalPoints: newPoints });

        console.log("✅ Quiz result saved");
        return { success: true };
    } catch (error) {
        console.error("❌ Quiz save error:", error);
        return { success: false, error: error.message };
    }
}

// ===========================================
// Student Profile Functions (Educational Data)
// ===========================================

// Save student educational profile to Firestore
export async function saveStudentProfileToFirestore(profileData) {
    if (!currentUser) {
        console.warn("⚠️ No user logged in, cannot save to Firestore");
        return { success: false, error: "Not logged in" };
    }

    try {
        // Build clean profile - only include relevant fields based on type
        const isUniversity = profileData.type === 'university' ||
            profileData.educationLevel === 'undergraduate' ||
            profileData.educationLevel === 'postgraduate' ||
            profileData.educationLevel === 'doctoral';

        const cleanProfile = {
            id: profileData.id,
            userId: currentUser.uid,
            email: currentUser.email,
            displayName: profileData.displayName,
            country: profileData.country,
            countryName: profileData.countryName,
            educationLevel: profileData.educationLevel,
            type: isUniversity ? 'university' : 'school',
            language: profileData.language || 'en-US',
            subjects: profileData.subjects || [],
            createdAt: profileData.createdAt,
            updatedAt: new Date()
        };

        // Add type-specific fields only
        if (isUniversity) {
            cleanProfile.department = profileData.department;
            cleanProfile.program = profileData.program;
            cleanProfile.programName = profileData.programName;
            cleanProfile.year = profileData.year;
            // Explicitly NOT including: board, class, stream
        } else {
            cleanProfile.board = profileData.board;
            cleanProfile.class = profileData.class;
            cleanProfile.stream = profileData.stream;
            // Explicitly NOT including: department, program, programName, year
        }

        console.log("📝 Saving clean profile to Firestore:", cleanProfile);

        // setDoc without merge option replaces the entire document
        await setDoc(doc(db, "studentProfiles", currentUser.uid), cleanProfile);

        console.log("✅ Student profile saved to Firestore:", currentUser.uid);
        return { success: true };
    } catch (error) {
        console.error("❌ Student profile save error:", error);
        return { success: false, error: error.message };
    }
}

// Load student educational profile from Firestore
export async function loadStudentProfileFromFirestore() {
    if (!currentUser) {
        return { success: false, error: "Not logged in", data: null };
    }

    try {
        const docSnap = await getDoc(doc(db, "studentProfiles", currentUser.uid));

        if (docSnap.exists()) {
            const profileData = docSnap.data();
            console.log("✅ Student profile loaded from Firestore:", currentUser.uid);
            return { success: true, data: profileData };
        } else {
            console.log("📝 No student profile found in Firestore for:", currentUser.uid);
            return { success: true, data: null };
        }
    } catch (error) {
        console.error("❌ Student profile load error:", error);
        return { success: false, error: error.message, data: null };
    }
}

// ===========================================
// UI Helper Functions
// ===========================================

export function showLoginForm() {
    // Update header to show login state
    updateHeaderAuthUI(null);
}

export function showUserProfile() {
    const authModal = document.getElementById('auth-modal');
    const loginForm = document.getElementById('login-form');
    const signupForm = document.getElementById('signup-form');
    const userProfileDiv = document.getElementById('user-profile');
    const userBtn = document.getElementById('user-btn');

    // Show auth modal with profile
    if (authModal) authModal.classList.remove('hidden');

    if (loginForm) loginForm.classList.add('hidden');
    if (signupForm) signupForm.classList.add('hidden');

    // Get student profile from localStorage
    let studentProfile = null;
    try {
        const savedProfile = localStorage.getItem('edumind_student_profile');
        if (savedProfile) {
            studentProfile = JSON.parse(savedProfile);
        }
    } catch (e) {
        console.error("Error loading student profile:", e);
    }

    if (currentUser && userProfileDiv) {
        const profileName = document.getElementById('profile-name');
        const profileEmail = document.getElementById('profile-email');
        const profileGrade = document.getElementById('profile-grade');
        const profileAvatarImg = document.getElementById('profile-avatar-img');
        const profileAvatarText = document.getElementById('profile-avatar-text');

        // Set name (from auth or student profile)
        const displayName = currentUser.displayName || studentProfile?.displayName || userProfile?.displayName || 'Student';
        if (profileName) profileName.textContent = displayName;
        if (profileEmail) profileEmail.textContent = currentUser.email || '';

        // Set grade/education info from student profile
        if (profileGrade && studentProfile) {
            const isUniversity = studentProfile.type === 'university' ||
                studentProfile.educationLevel === 'undergraduate' ||
                studentProfile.educationLevel === 'postgraduate' ||
                studentProfile.educationLevel === 'doctoral';

            if (isUniversity) {
                // Show program code (CSE, EEE, etc.) or department name
                const programDisplay = studentProfile.programName ||
                    (studentProfile.program ? studentProfile.program.toUpperCase() : '') ||
                    studentProfile.department ||
                    'University';
                profileGrade.textContent = `${programDisplay} - Year ${studentProfile.year || 1}`;
            } else {
                profileGrade.textContent = `Class ${studentProfile.class || 10} - ${(studentProfile.stream || 'Science').charAt(0).toUpperCase() + (studentProfile.stream || 'science').slice(1)}`;
            }
        }

        // Handle profile picture
        if (currentUser.photoURL && profileAvatarImg) {
            profileAvatarImg.src = currentUser.photoURL;
            profileAvatarImg.style.display = 'block';
            if (profileAvatarText) profileAvatarText.style.display = 'none';
        } else {
            if (profileAvatarImg) profileAvatarImg.style.display = 'none';
            if (profileAvatarText) {
                profileAvatarText.style.display = 'block';
                profileAvatarText.textContent = displayName ? displayName.charAt(0).toUpperCase() : '👤';
            }
        }

        // Set stats
        const totalPoints = userProfile?.totalPoints || studentProfile?.totalPoints || 0;
        const streak = userProfile?.streak || studentProfile?.streak || 0;
        const topicsCount = userProfile?.subjects?.length || studentProfile?.subjects?.length || 0;

        const profilePoints = document.getElementById('profile-points');
        const profileStreak = document.getElementById('profile-streak');
        const profileTopics = document.getElementById('profile-topics');

        if (profilePoints) profilePoints.textContent = totalPoints;
        if (profileStreak) profileStreak.textContent = streak;
        if (profileTopics) profileTopics.textContent = topicsCount;

        // Show/hide buttons based on login state
        const linkAccountBtn = document.getElementById('link-account-btn');
        const logoutBtn = document.getElementById('logout-btn');
        const changePhotoBtn = document.getElementById('change-profile-pic-btn');

        if (linkAccountBtn) linkAccountBtn.classList.add('hidden'); // Hide link account for logged in users
        if (logoutBtn) logoutBtn.classList.remove('hidden');
        if (changePhotoBtn) changePhotoBtn.classList.remove('hidden');

        userProfileDiv.classList.remove('hidden');
    } else if (!currentUser) {
        // Not logged in but might have local profile
        if (studentProfile && userProfileDiv) {
            const profileName = document.getElementById('profile-name');
            const profileEmail = document.getElementById('profile-email');
            const profileGrade = document.getElementById('profile-grade');
            const profileAvatarText = document.getElementById('profile-avatar-text');
            const profileAvatarImg = document.getElementById('profile-avatar-img');

            if (profileName) profileName.textContent = studentProfile.displayName || 'Student';
            if (profileEmail) profileEmail.textContent = studentProfile.email || 'Local profile (not synced)';

            if (profileGrade) {
                const isUniversity = studentProfile.type === 'university' ||
                    studentProfile.educationLevel === 'undergraduate' ||
                    studentProfile.educationLevel === 'postgraduate' ||
                    studentProfile.educationLevel === 'doctoral';

                if (isUniversity) {
                    const programDisplay = studentProfile.programName ||
                        (studentProfile.program ? studentProfile.program.toUpperCase() : '') ||
                        studentProfile.department ||
                        'University';
                    profileGrade.textContent = `${programDisplay} - Year ${studentProfile.year || 1}`;
                } else {
                    profileGrade.textContent = `Class ${studentProfile.class || 10} - ${(studentProfile.stream || 'Science').charAt(0).toUpperCase() + (studentProfile.stream || 'science').slice(1)}`;
                }
            }

            if (profileAvatarImg) profileAvatarImg.style.display = 'none';
            if (profileAvatarText) {
                profileAvatarText.style.display = 'block';
                profileAvatarText.textContent = studentProfile.displayName ? studentProfile.displayName.charAt(0).toUpperCase() : '👤';
            }

            // Show/hide buttons for local-only users
            const linkAccountBtn = document.getElementById('link-account-btn');
            const logoutBtn = document.getElementById('logout-btn');
            const changePhotoBtn = document.getElementById('change-profile-pic-btn');

            if (linkAccountBtn) linkAccountBtn.classList.remove('hidden'); // Show link account for local users
            if (logoutBtn) {
                logoutBtn.textContent = 'Clear Profile';
                logoutBtn.classList.remove('hidden');
            }
            if (changePhotoBtn) changePhotoBtn.classList.add('hidden'); // Can't upload without account

            // Set stats
            const profilePoints = document.getElementById('profile-points');
            const profileStreak = document.getElementById('profile-streak');
            const profileTopics = document.getElementById('profile-topics');

            if (profilePoints) profilePoints.textContent = studentProfile.totalPoints || 0;
            if (profileStreak) profileStreak.textContent = studentProfile.streak || 0;
            if (profileTopics) profileTopics.textContent = studentProfile.subjects?.length || 0;

            userProfileDiv.classList.remove('hidden');
        }
    }

    if (userBtn) userBtn.textContent = currentUser?.displayName?.split(' ')[0] || userProfile?.displayName?.split(' ')[0] || 'Profile';
}

console.log("✅ Auth module loaded");

// ===========================================
// Textbook Library Functions
// ===========================================

// Get textbooks for a specific class/stream
export async function getTextbooks(filters = {}) {
    try {
        const textbooksRef = collection(db, 'textbooks');
        let q;

        // Build query based on filters
        if (filters.department && filters.program) {
            // University query
            q = query(textbooksRef,
                where('type', '==', 'university'),
                where('department', '==', filters.department),
                where('program', '==', filters.program)
            );
        } else if (filters.classNum && filters.stream) {
            // School query with stream
            q = query(textbooksRef,
                where('class', '==', parseInt(filters.classNum)),
                where('stream', 'in', [filters.stream, '', null])
            );
        } else if (filters.classNum) {
            // School query without stream
            q = query(textbooksRef, where('class', '==', parseInt(filters.classNum)));
        } else {
            // Get all (limited)
            q = query(textbooksRef, limit(50));
        }

        const snapshot = await getDocs(q);
        const textbooks = [];
        snapshot.forEach(doc => {
            textbooks.push({ id: doc.id, ...doc.data() });
        });

        return textbooks;
    } catch (error) {
        console.error("Error getting textbooks:", error);
        return [];
    }
}

// Check if textbook already exists
export async function checkTextbookExists(classNum, subject, board = 'NCTB') {
    try {
        const textbooksRef = collection(db, 'textbooks');
        const q = query(textbooksRef,
            where('class', '==', parseInt(classNum)),
            where('subject', '==', subject.toLowerCase()),
            where('board', '==', board)
        );

        const snapshot = await getDocs(q);
        return !snapshot.empty;
    } catch (error) {
        console.error("Error checking textbook:", error);
        return false;
    }
}

// Check if university textbook already exists
export async function checkUniversityTextbookExists(department, program, subject) {
    try {
        const textbooksRef = collection(db, 'textbooks');
        const q = query(textbooksRef,
            where('type', '==', 'university'),
            where('department', '==', department),
            where('program', '==', program),
            where('subject', '==', subject.toLowerCase())
        );

        const snapshot = await getDocs(q);
        return !snapshot.empty;
    } catch (error) {
        console.error("Error checking university textbook:", error);
        return false;
    }
}

// Upload textbook to Firebase Storage and create Firestore entry
export async function uploadTextbook(file, metadata, onProgress) {
    try {
        const user = currentUser;
        const isUniversity = metadata.type === 'university';

        // Different file path for university vs school
        const fileName = isUniversity
            ? `textbooks/university/${metadata.department}/${metadata.program}/${metadata.subject}-${Date.now()}.pdf`
            : `textbooks/class-${metadata.class}/${metadata.subject}-${Date.now()}.pdf`;

        const storageRef = ref(storage, fileName);

        // Upload file
        const uploadTask = await uploadBytes(storageRef, file);

        // Get download URL
        const downloadUrl = await getDownloadURL(storageRef);

        // Create Firestore document - different structure for university
        let textbookData;

        if (isUniversity) {
            textbookData = {
                title: metadata.title,
                subject: metadata.subject.toLowerCase(),
                type: 'university',
                department: metadata.department,
                program: metadata.program,
                country: metadata.country || 'bangladesh',
                fileName: file.name,
                fileSize: file.size,
                pdfUrl: downloadUrl,
                storagePath: fileName,
                uploadedBy: user?.uid || 'anonymous',
                uploadedAt: new Date().toISOString(),
                verified: false,
                chaptersCount: 0,
                chaptersExtracted: false
            };
        } else {
            textbookData = {
                title: metadata.title,
                subject: metadata.subject.toLowerCase(),
                class: parseInt(metadata.class),
                stream: metadata.stream || '',
                board: metadata.board || 'NCTB',
                country: metadata.country || 'bangladesh',
                fileName: file.name,
                fileSize: file.size,
                pdfUrl: downloadUrl,
                storagePath: fileName,
                uploadedBy: user?.uid || 'anonymous',
                uploadedAt: new Date().toISOString(),
                verified: false,
                chaptersCount: 0,
                chaptersExtracted: false
            };
        }

        const docRef = await addDoc(collection(db, 'textbooks'), textbookData);

        return {
            success: true,
            textbookId: docRef.id,
            downloadUrl
        };
    } catch (error) {
        console.error("Error uploading textbook:", error);
        return { success: false, error: error.message };
    }
}

// Save extracted chapters for a textbook
export async function saveTextbookChapters(textbookId, chapters) {
    try {
        // Save each chapter
        for (const chapter of chapters) {
            await addDoc(collection(db, 'textbook_chapters'), {
                bookId: textbookId,
                chapterNum: chapter.chapterNum,
                title: chapter.title,
                content: chapter.content,
                keywords: chapter.keywords || [],
                summary: chapter.summary || '',
                createdAt: new Date().toISOString()
            });
        }

        // Update textbook document with chapters count
        const textbookRef = doc(db, 'textbooks', textbookId);
        await updateDoc(textbookRef, {
            chaptersCount: chapters.length,
            chaptersExtracted: true
        });

        return { success: true };
    } catch (error) {
        console.error("Error saving chapters:", error);
        return { success: false, error: error.message };
    }
}

// Get chapters for a textbook
export async function getTextbookChapters(textbookId) {
    try {
        console.log("Fetching chapters for book:", textbookId);
        const startTime = Date.now();

        const chaptersRef = collection(db, 'textbook_chapters');

        // First, get all chapters to debug
        const allSnapshot = await getDocs(chaptersRef);
        console.log(`📚 Total chapters in collection: ${allSnapshot.size}`);
        if (allSnapshot.size > 0 && allSnapshot.size < 10) {
            allSnapshot.forEach(doc => {
                console.log(`  Chapter ${doc.id}:`, doc.data().bookId, 'vs', textbookId);
            });
        }

        // Now query for specific book
        const q = query(chaptersRef, where('bookId', '==', textbookId));
        const snapshot = await getDocs(q);
        console.log(`Query took ${Date.now() - startTime}ms, found ${snapshot.size} chapters`);

        const chapters = [];
        snapshot.forEach(doc => {
            chapters.push({ id: doc.id, ...doc.data() });
        });

        // Sort locally instead of in Firestore
        chapters.sort((a, b) => (a.chapterNum || 0) - (b.chapterNum || 0));

        return chapters;
    } catch (error) {
        console.error("Error getting chapters:", error);
        return [];
    }
}

// Search chapters by keyword/topic
export async function searchChapters(query, filters = {}) {
    try {
        // Get all chapters for the class/subject
        const chaptersRef = collection(db, 'textbook_chapters');
        let q;

        if (filters.bookId) {
            q = query(chaptersRef, where('bookId', '==', filters.bookId));
        } else {
            q = query(chaptersRef, limit(100));
        }

        const snapshot = await getDocs(q);
        const chapters = [];
        const searchLower = query.toLowerCase();

        snapshot.forEach(doc => {
            const data = doc.data();
            // Search in title, content, keywords
            if (data.title?.toLowerCase().includes(searchLower) ||
                data.content?.toLowerCase().includes(searchLower) ||
                data.keywords?.some(k => k.toLowerCase().includes(searchLower))) {
                chapters.push({ id: doc.id, ...data });
            }
        });

        return chapters;
    } catch (error) {
        console.error("Error searching chapters:", error);
        return [];
    }
}



```

## Source file: api/tts.js

{
  "path": "api/tts.js",
  "blob_sha": "bd02d3797fc739e50f0348f6e2e3d378c196b52c",
  "sha256": "05c818c8426f63e00754e2152fe0cfe33e79256db2c2fa25a5f82efd358aab05",
  "lines": 76,
  "bytes": 2503,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1
  },
  "url": "https://github.com/abubokkor-cse/EduMind-AI-Virtual-Teacher/blob/73781e968856caa67d28c3251b318e43e90dce56/api/tts.js",
  "function_signature_candidates": []
}

```text

// Vercel Serverless Function - ElevenLabs TTS
// Converts text to Bengali speech securely

module.exports = async function handler(req, res) {
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Methods', 'POST, OPTIONS');
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type');

    if (req.method === 'OPTIONS') {
        return res.status(200).end();
    }

    if (req.method !== 'POST') {
        return res.status(405).json({ error: 'Method not allowed' });
    }

    const ELEVENLABS_API_KEY = process.env.ELEVENLABS_API_KEY;

    if (!ELEVENLABS_API_KEY) {
        return res.status(500).json({ error: 'ElevenLabs API key not configured' });
    }

    try {
        const { text, voiceId, modelId } = req.body;

        if (!text) {
            return res.status(400).json({ error: 'Text is required' });
        }

        // Default voice ID for Bengali
        const voice = voiceId || 'nPczCjzI2devNBz1zQrb'; // Brian voice or your preferred
        const model = modelId || 'eleven_multilingual_v2';

        const url = `https://api.elevenlabs.io/v1/text-to-speech/${voice}`;

        const response = await fetch(url, {
            method: 'POST',
            headers: {
                'Accept': 'audio/mpeg',
                'Content-Type': 'application/json',
                'xi-api-key': ELEVENLABS_API_KEY
            },
            body: JSON.stringify({
                text,
                model_id: model,
                voice_settings: {
                    stability: 0.5,
                    similarity_boost: 0.75,
                    style: 0.5,
                    use_speaker_boost: true
                }
            })
        });

        if (!response.ok) {
            const errorText = await response.text();
            console.error('ElevenLabs Error:', errorText);
            return res.status(response.status).json({ error: 'TTS generation failed', details: errorText });
        }

        // Get audio as buffer
        const audioBuffer = await response.arrayBuffer();

        // Return as base64 for easy handling in frontend
        const base64Audio = Buffer.from(audioBuffer).toString('base64');

        return res.status(200).json({
            audio: base64Audio,
            contentType: 'audio/mpeg'
        });

    } catch (error) {
        console.error('TTS Error:', error);
        return res.status(500).json({ error: 'Failed to generate speech', details: error.message });
    }
}


```
